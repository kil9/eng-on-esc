// macOS 용 eng-on-esc
// ESC 키를 누르면 한국어 IME를 영문 모드로 강제 전환.
// ESC 본래 동작(취소/닫기 등)은 그대로 통과시킴.
//
// ── macOS 한국어 IME 구조 ──────────────────────────────────────────────────
// macOS 에서 한국어 IME 는 Caps Lock 키로 한글/영문 모드를 토글한다.
// ESC 눌렀을 때 한국어 모드이면 Caps Lock 을 시뮬레이션해 영문 전환.
// OS API 로 현재 IME 모드를 신뢰성 있게 읽을 수 없어 Windows 버전과 동일하게
// 자체 상태 머신으로 추적한다.
//
// ── 상태 추적 ────────────────────────────────────────────────────────────
// KOREAN_MODE: 전역 bool (false=영문, true=한국어).
// LL 후킹에서 사용자가 직접 누른 Caps Lock 을 감지하면 상태 반전.
// 우리가 simulate 로 보낸 Caps Lock 은 SENDING_CAPS 플래그로 구분해 무시.
//
// ── ESC 처리 흐름 ────────────────────────────────────────────────────────
// 1. keyboard_callback 에서 ESC KeyPress 감지 → 워커 채널로 신호.
// 2. ESC 이벤트는 Some(event) 로 그대로 통과.
// 3. 워커가 5ms 디바운스 후 KOREAN_MODE 확인.
// 4. 한국어 모드이면 KOREAN_MODE = false 로 set, Caps Lock 시뮬레이션 전송.
// 5. 영문 모드이면 아무 동작 없음.
//
// ── 자동 시작 ────────────────────────────────────────────────────────────
// ~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist 생성/삭제로 관리.
//
// ── 실행 권한 ────────────────────────────────────────────────────────────
// rdev::grab 은 CGEventTap 을 사용하며 Accessibility(손쉬운 사용) 권한이 필요.
//   시스템 환경설정 → 개인 정보 보호 및 보안 → 손쉬운 사용 → 이 앱 허용

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use rdev::{grab, simulate, Event, EventType, Key};

// ── Accessibility 권한 확인 ───────────────────────────────────────────────

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// 현재 프로세스가 Accessibility(손쉬운 사용) 권한을 가졌는지 확인.
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

/// Accessibility 권한이 없으면 시스템 권한 요청 다이얼로그를 띄우고 false 반환.
fn ensure_accessibility() -> bool {
    // kAXTrustedCheckOptionPrompt = true 로 CoreFoundation 딕셔너리 생성.
    // 간단히 CFDictionaryCreateMutable 없이 raw 포인터 NULL 로 호출하면
    // 다이얼로그 없이 현재 상태만 조회한다.
    // 여기서는 prompt=true 동작을 위해 직접 CFDictionary 를 구성한다.
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let val = CFBoolean::true_value();
    let dict: CFDictionary<CFString, CFBoolean> =
        CFDictionary::from_CFType_pairs(&[(key, val)]);

    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as *const _) }
}
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

/// 현재 IME 모드 (false=영문, true=한국어).
/// 전역 단일 값 — macOS 는 IME 상태가 thread-local 이 아님.
static KOREAN_MODE: OnceLock<Mutex<bool>> = OnceLock::new();

/// ESC 신호를 워커 스레드로 전달하는 채널 송신단.
static SENDER: OnceLock<Sender<()>> = OnceLock::new();

/// simulate 로 보낸 Caps Lock 이 다시 callback 으로 들어올 때 무시하기 위한 플래그.
static SENDING_CAPS: AtomicBool = AtomicBool::new(false);

fn korean_mode() -> &'static Mutex<bool> {
    KOREAN_MODE.get_or_init(|| Mutex::new(false))
}

// ── 자동 시작 (LaunchAgent) ───────────────────────────────────────────────

const PLIST_LABEL: &str = "io.github.kil9.eng-on-esc";

fn plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{PLIST_LABEL}.plist"))
}

fn autostart_is_enabled() -> bool {
    plist_path().exists()
}

fn autostart_enable() {
    let exe = std::env::current_exe().unwrap_or_default();
    let content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
           <key>Label</key><string>{PLIST_LABEL}</string>\n\
           <key>ProgramArguments</key>\n\
           <array><string>{exe}</string></array>\n\
           <key>RunAtLoad</key><true/>\n\
         </dict>\n\
         </plist>\n",
        exe = exe.display()
    );
    let path = plist_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, content);
}

fn autostart_disable() {
    let _ = fs::remove_file(plist_path());
}

// ── 메뉴바 아이콘 ─────────────────────────────────────────────────────────

/// 22×22 template 아이콘 생성 (원형, 흑백 마스크).
/// with_icon_as_template(true) 와 함께 사용하면 macOS 가 다크/라이트 모드에 맞게 렌더링.
fn make_icon() -> Icon {
    const SIZE: usize = 22;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let cx = (SIZE as f32 - 1.0) / 2.0;
    let cy = cx;
    let r = 7.5f32;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let alpha = if (dx * dx + dy * dy).sqrt() <= r { 255u8 } else { 0u8 };
            let i = (y * SIZE + x) * 4;
            // R=0, G=0, B=0 (검정), A=alpha — template 모드에서 색상은 시스템이 재지정
            rgba[i + 3] = alpha;
        }
    }
    Icon::from_rgba(rgba, SIZE as u32, SIZE as u32).unwrap()
}

// ── 키보드 후킹 콜백 ──────────────────────────────────────────────────────

/// 글로벌 키보드 후킹 콜백.
/// `Some(event)` 반환 → 이벤트 시스템으로 그대로 전달.
/// `None` 반환 → 이벤트 삼킴 (앱에 전달되지 않음).
fn keyboard_callback(event: Event) -> Option<Event> {
    match event.event_type {
        // 사용자가 직접 누른 Caps Lock → 한/영 모드 반전.
        EventType::KeyPress(Key::CapsLock) => {
            if !SENDING_CAPS.load(Ordering::SeqCst) {
                let mut mode = korean_mode().lock().unwrap();
                *mode = !*mode;
            }
            Some(event)
        }

        // ESC → 워커에 신호, 이벤트 그대로 통과.
        EventType::KeyPress(Key::Escape) => {
            if let Some(tx) = SENDER.get() {
                let _ = tx.send(());
            }
            Some(event)
        }

        _ => Some(event),
    }
}

fn main() {
    // Accessibility 권한 확인 — 없으면 시스템 다이얼로그를 띄우고 종료.
    // 권한을 허용한 뒤 앱을 다시 실행해야 한다.
    if !ensure_accessibility() {
        eprintln!(
            "손쉬운 사용 권한이 필요합니다.\n\
             시스템 설정 → 개인 정보 보호 및 보안 → 손쉬운 사용에서 이 앱을 허용한 뒤 다시 실행하세요."
        );
        // 다이얼로그가 열릴 시간을 잠깐 준다.
        thread::sleep(Duration::from_secs(1));
        std::process::exit(1);
    }

    let (tx, rx) = mpsc::channel::<()>();
    SENDER.set(tx).expect("SENDER 초기화 실패");

    // ESC 처리 워커 스레드.
    thread::spawn(move || loop {
        if rx.recv().is_err() {
            break;
        }
        // ESC 연타 디바운스
        while rx.recv_timeout(Duration::from_millis(5)).is_ok() {}

        let needs_send = {
            let mut mode = korean_mode().lock().unwrap();
            if *mode {
                *mode = false;
                true
            } else {
                false
            }
        };

        if needs_send {
            SENDING_CAPS.store(true, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(10));
            let _ = simulate(&EventType::KeyPress(Key::CapsLock));
            thread::sleep(Duration::from_millis(10));
            let _ = simulate(&EventType::KeyRelease(Key::CapsLock));
            thread::sleep(Duration::from_millis(10));
            SENDING_CAPS.store(false, Ordering::SeqCst);
        }
    });

    // tao 이벤트 루프를 먼저 생성해 NSApplication 을 초기화한다.
    // TrayIconBuilder 가 NSStatusItem 을 생성할 때 NSApplication 이 필요하므로
    // EventLoop::new() 보다 늦게 호출해야 한다.
    let mut event_loop = EventLoopBuilder::<()>::new().build();
    // Dock 아이콘 없이 메뉴바 전용 앱으로 동작.
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    // 메뉴바 아이콘 + 메뉴 구성 (NSApplication 초기화 이후).
    let menu = Menu::new();
    let autostart_item =
        CheckMenuItem::new("시작 시 자동 실행", true, autostart_is_enabled(), None);
    let quit_item = MenuItem::new("종료", true, None);
    menu.append_items(&[
        &autostart_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])
    .unwrap();

    let autostart_id = autostart_item.id().clone();
    let quit_id = quit_item.id().clone();

    // TrayIcon 을 스코프에 유지 — drop 되면 메뉴바 아이콘이 사라짐.
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("eng-on-esc")
        .with_icon(make_icon())
        .with_icon_as_template(true)
        .build()
        .unwrap();

    // 글로벌 키보드 후킹을 별도 스레드에서 실행.
    // (메인 스레드는 tao 이벤트 루프가 점유하므로)
    thread::spawn(|| {
        if let Err(err) = grab(keyboard_callback) {
            eprintln!("키보드 후킹 실패: {err:?}");
            std::process::exit(1);
        }
    });

    event_loop.run(move |_event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == quit_id {
                *control_flow = ControlFlow::Exit;
            } else if ev.id == autostart_id {
                if autostart_is_enabled() {
                    autostart_disable();
                    autostart_item.set_checked(false);
                } else {
                    autostart_enable();
                    autostart_item.set_checked(true);
                }
            }
        }
    });
}
