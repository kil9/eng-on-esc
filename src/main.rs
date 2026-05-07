// ESC 키를 누르면 한국어 IME를 무조건 영문 모드로 강제 전환.
// ESC 본래 동작(취소/닫기 등)은 그대로 통과시킴.
// 트레이 아이콘에서 좌/우클릭으로 종료 메뉴를 띄울 수 있음.

#![windows_subsystem = "windows"]

use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::Ime::{
    ImmGetContext, ImmGetDefaultIMEWnd, ImmReleaseContext, ImmSetOpenStatus,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VK_ESCAPE, VK_HANGUL,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetMessageW,
    GetWindowThreadProcessId, LoadIconW, PostQuitMessage, RegisterClassW, SendMessageTimeoutW,
    SetForegroundWindow, SetWindowsHookExW, TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx,
    GUITHREADINFO, HHOOK, HMENU, IDI_APPLICATION, KBDLLHOOKSTRUCT, MF_STRING, MSG, SMTO_ABORTIFHUNG,
    TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WM_COMMAND, WM_DESTROY,
    WM_KEYDOWN, WM_LBUTTONUP, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_USER, WNDCLASSW, WS_OVERLAPPED,
};

const WM_IME_CONTROL: u32 = 0x0283;
const IMC_SETOPENSTATUS: usize = 0x0006;
const IMC_GETOPENSTATUS: usize = 0x0005;

fn dbg_log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("esc-eng-ime.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = writeln!(f, "[{}] {}", now, msg);
    }
}
const WM_TRAYICON: u32 = WM_USER + 1;
const IDM_EXIT: u32 = 1001;
const TRAY_UID: u32 = 1;

static HOOK: OnceLock<isize> = OnceLock::new();
static SENDER: OnceLock<Sender<()>> = OnceLock::new();

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let m = wparam.0 as u32;
        let is_down = m == WM_KEYDOWN || m == WM_SYSKEYDOWN;
        if is_down && kb.vkCode == VK_ESCAPE.0 as u32 {
            if let Some(tx) = SENDER.get() {
                let _ = tx.send(());
            }
        }
    }
    let h = HHOOK(HOOK.get().copied().unwrap_or(0) as *mut _);
    CallNextHookEx(h, code, wparam, lparam)
}

unsafe fn force_english_ime() {
    let fg = GetForegroundWindow();
    if fg.0.is_null() {
        dbg_log("force: fg=NULL");
        return;
    }

    // 방법 1: AttachThreadInput 후, 포커스된 컨트롤(자식 윈도우)의 IME context를 얻어
    // ImmSetOpenStatus로 직접 영문 강제. 다른 프로세스의 hImc는 본래 못 만지지만
    // 입력 큐 attach 후엔 가능.
    let target_tid = GetWindowThreadProcessId(fg, None);
    let our_tid = GetCurrentThreadId();
    let mut imc_ok = false;
    let mut himc_addr: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut focus_hwnd_addr: *mut std::ffi::c_void = std::ptr::null_mut();
    if target_tid != 0 {
        let attached = if target_tid != our_tid {
            AttachThreadInput(our_tid, target_tid, true).as_bool()
        } else {
            true
        };
        if attached {
            // attach 후 GetGUIThreadInfo로 포커스 hwnd 얻기.
            let mut info: GUITHREADINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
            let focus_hwnd = if GetGUIThreadInfo(target_tid, &mut info).is_ok()
                && !info.hwndFocus.0.is_null()
            {
                info.hwndFocus
            } else {
                fg
            };
            focus_hwnd_addr = focus_hwnd.0;
            let himc = ImmGetContext(focus_hwnd);
            himc_addr = himc.0;
            if !himc.0.is_null() {
                imc_ok = ImmSetOpenStatus(himc, BOOL(0)).as_bool();
                let _ = ImmReleaseContext(focus_hwnd, himc);
            }
            if target_tid != our_tid {
                let _ = AttachThreadInput(our_tid, target_tid, false);
            }
        }
    }

    // 방법 2: default IME 윈도우에 WM_IME_CONTROL 송신 (백업).
    let ime = ImmGetDefaultIMEWnd(fg);
    let mut send_lresult: isize = 0;
    let mut send_result: usize = 0;
    if !ime.0.is_null() {
        let r = SendMessageTimeoutW(
            ime,
            WM_IME_CONTROL,
            WPARAM(IMC_SETOPENSTATUS),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            100,
            Some(&mut send_result as *mut usize),
        );
        send_lresult = r.0;
    }

    dbg_log(&format!(
        "force: fg={:p} tid={} our_tid={} focus={:p} himc={:p} imc_ok={} ime={:p} lresult={} result={}",
        fg.0, target_tid, our_tid, focus_hwnd_addr, himc_addr, imc_ok, ime.0, send_lresult, send_result
    ));
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
    // 메뉴 밖을 클릭했을 때 사라지려면 포그라운드여야 함.
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

    dbg_log("main: starting");
    thread::spawn(move || loop {
        if rx.recv().is_err() {
            break;
        }
        while rx.recv_timeout(Duration::from_millis(5)).is_ok() {}
        dbg_log("worker: ESC received");
        unsafe { force_english_ime() };
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
        dbg_log(&format!("main: hook installed = {:p}", hook.0));

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook);
    }
    Ok(())
}
