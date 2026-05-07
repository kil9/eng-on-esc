// ESC 키를 누르면 한국어 IME를 영문 모드로 강제 전환.
// ESC 본래 동작(취소/닫기 등)은 그대로 통과시킴.
// 트레이 아이콘에서 좌/우클릭으로 종료 메뉴.
//
// IME 상태는 OS API에 의존하지 않고 자체 상태 머신으로 추적.
// Windows IME 상태는 thread-local이므로 thread ID 별로 분리 추적:
//  - 새로 만난 thread는 영문이라 가정
//  - LL hook이 사용자가 직접 누른 한/영키(VK_HANGUL)를 감지하면
//    그 시점의 포그라운드 윈도우 thread의 상태를 토글
//  - ESC 시 그 thread의 상태가 한국어이면 SendInput VK_HANGUL로 영문 전환 + 상태 갱신
// 처음 만나는 thread가 이미 한국어 모드면 첫 ESC는 무동작 → 한 번 한/영키 눌러 동기화.

#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_ESCAPE, VK_HANGUL,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    LoadIconW, PostQuitMessage, RegisterClassW, SetForegroundWindow, SetWindowsHookExW,
    TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, HHOOK, HMENU, IDI_APPLICATION,
    KBDLLHOOKSTRUCT, MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, WH_KEYBOARD_LL,
    WINDOW_EX_STYLE, WM_COMMAND, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONUP, WM_RBUTTONUP,
    WM_SYSKEYDOWN, WM_USER, WNDCLASSW, WS_OVERLAPPED,
};

const LLKHF_INJECTED: u32 = 0x10;
const WM_TRAYICON: u32 = WM_USER + 1;
const IDM_EXIT: u32 = 1001;
const TRAY_UID: u32 = 1;

static STATES: OnceLock<Mutex<HashMap<u32, bool>>> = OnceLock::new();
static HOOK: OnceLock<isize> = OnceLock::new();
static SENDER: OnceLock<Sender<()>> = OnceLock::new();

fn states() -> &'static Mutex<HashMap<u32, bool>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe fn foreground_tid() -> u32 {
    let fg = GetForegroundWindow();
    if fg.0.is_null() {
        return 0;
    }
    GetWindowThreadProcessId(fg, None)
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let m = wparam.0 as u32;
        if m == WM_KEYDOWN || m == WM_SYSKEYDOWN {
            let vk = kb.vkCode;
            let injected = (kb.flags.0 & LLKHF_INJECTED) != 0;

            // 사용자가 직접 누른 한/영키이면 그 thread의 내부 상태 토글.
            // INJECTED는 우리가 send_vk로 보낸 것이라 별도 경로에서 갱신함.
            if !injected && vk == VK_HANGUL.0 as u32 {
                let tid = foreground_tid();
                if tid != 0 {
                    let mut s = states().lock().unwrap();
                    let v = s.entry(tid).or_insert(false);
                    *v = !*v;
                }
            }

            if vk == VK_ESCAPE.0 as u32 {
                if let Some(tx) = SENDER.get() {
                    let _ = tx.send(());
                }
            }
        }
    }
    let h = HHOOK(HOOK.get().copied().unwrap_or(0) as *mut _);
    CallNextHookEx(h, code, wparam, lparam)
}

unsafe fn send_vk(vk: VIRTUAL_KEY) {
    let mut inputs: [INPUT; 2] = std::mem::zeroed();
    inputs[0].r#type = INPUT_KEYBOARD;
    inputs[0].Anonymous = INPUT_0 {
        ki: KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: KEYBD_EVENT_FLAGS(0),
            time: 0,
            dwExtraInfo: 0,
        },
    };
    inputs[1] = inputs[0];
    inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_TRAYICON {
        let event = (lparam.0 as u32) & 0xFFFF;
        if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
            show_context_menu(hwnd);
        }
        return LRESULT(0);
    }
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 as u32) & 0xFFFF;
            if id == IDM_EXIT {
                remove_tray(hwnd);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_context_menu(hwnd: HWND) {
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let menu = match CreatePopupMenu() {
        Ok(m) => m,
        Err(_) => return,
    };
    let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT as usize, w!("종료(&X)"));
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
        pt.x,
        pt.y,
        0,
        hwnd,
        None,
    );
    let _ = DestroyMenu(menu);
}

unsafe fn add_tray(hwnd: HWND) {
    let icon = LoadIconW(HINSTANCE::default(), IDI_APPLICATION).unwrap_or_default();
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon;
    let tip: Vec<u16> = "ESC -> 영문 IME (클릭: 종료 메뉴)".encode_utf16().collect();
    for (i, c) in tip.iter().take(127).enumerate() {
        nid.szTip[i] = *c;
    }
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn remove_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}

fn main() -> windows::core::Result<()> {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = SENDER.set(tx);

    thread::spawn(move || loop {
        if rx.recv().is_err() {
            break;
        }
        // ESC 연타 시 짧은 디바운스
        while rx.recv_timeout(Duration::from_millis(5)).is_ok() {}

        let tid = unsafe { foreground_tid() };
        if tid == 0 {
            continue;
        }
        let needs_send = {
            let mut s = states().lock().unwrap();
            let v = s.entry(tid).or_insert(false);
            if *v {
                *v = false;
                true
            } else {
                false
            }
        };
        if needs_send {
            unsafe { send_vk(VK_HANGUL) };
        }
    });

    unsafe {
        let hmodule = GetModuleHandleW(None)?;
        let hinst = HINSTANCE(hmodule.0);

        let class_name = w!("EscEngImeTrayWnd");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinst;
        wc.lpszClassName = class_name;
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("esc-eng-ime"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND::default(),
            HMENU::default(),
            hinst,
            None,
        )?;

        add_tray(hwnd);

        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinst, 0)?;
        let _ = HOOK.set(hook.0 as isize);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook);
    }
    Ok(())
}
