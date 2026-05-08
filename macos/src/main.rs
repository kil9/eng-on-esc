// macOS 용 eng-on-esc
// ESC 키를 누르면 한국어 IME를 영문 모드로 강제 전환.
// ESC 본래 동작(취소/닫기 등)은 그대로 통과시킴.
//
// ── macOS 한국어 IME 구조 ──────────────────────────────────────────────────
// macOS 에서 한국어 IME 는 Caps Lock 키로 한글/영문 모드를 토글한다.
// (Windows 의 VK_HANGUL 과 동일한 역할)
//
// ESC 눌렀을 때 한국어 모드이면 Caps Lock 을 시뮬레이션해 영문 전환.
// OS API 로 현재 IME 모드를 신뢰성 있게 읽을 수 없어 Windows 버전과 동일하게
// 자체 상태 머신으로 추적한다.
//
// ── 상태 추적 ────────────────────────────────────────────────────────────
// KOREAN_MODE: 전역 bool (false=영문, true=한국어).
// LL 후킹에서 사용자가 직접 누른 Caps Lock 을 감지하면 상태 반전.
// 우리가 simulate 로 보낸 Caps Lock 은 SENDING_CAPS 플래그로 구분해 무시.
// (Windows 의 LLKHF_INJECTED 대응)
//
// ── ESC 처리 흐름 ────────────────────────────────────────────────────────
// 1. keyboard_callback 에서 ESC KeyPress 감지 → 워커 채널로 신호.
// 2. ESC 이벤트는 Some(event) 로 그대로 통과.
// 3. 워커가 5ms 디바운스 후 KOREAN_MODE 확인.
// 4. 한국어 모드이면 KOREAN_MODE = false 로 set, Caps Lock 시뮬레이션 전송.
// 5. 영문 모드이면 아무 동작 없음.
//
// ── 한계 ────────────────────────────────────────────────────────────────
// - 앱 시작 시 한국어 모드인 경우 첫 ESC 는 무동작. 한 번 Caps Lock 으로 동기화.
// - 마우스 클릭으로 IME 전환, 입력 소스 직접 변경 등은 추적 불가.
// - Caps Lock 을 일반 영문 대소문자 토글로 쓰는 환경에는 맞지 않는다.
//   (한국어 IME 가 설치된 macOS 의 Caps Lock = 한/영 전용으로 작동하는 환경 기준)
//
// ── 실행 권한 ────────────────────────────────────────────────────────────
// rdev::grab 은 CGEventTap 을 사용하며 Accessibility(손쉬운 사용) 권한이 필요.
//   시스템 환경설정 → 개인 정보 보호 및 보안 → 손쉬운 사용 → 이 앱 허용
// 권한 없이 실행하면 에러 메시지를 출력하고 종료한다.
//
// TODO: 트레이(메뉴바) 아이콘 구현
//   - tray-icon + tao 를 이용해 NSStatusItem 으로 메뉴바 상주.
//   - tao 이벤트 루프는 메인 스레드를 점유해야 하므로,
//     현재 main 에서 grab 을 직접 호출하는 구조를 바꿔야 한다:
//     grab 을 spawn_blocking / 별도 thread 로 이동하고,
//     메인 스레드에서 event_loop.run() 실행.
//   - 자동 시작: ~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist 생성/삭제.
//
// TODO: 입력 소스 직접 확인
//   core-foundation crate + TIS C API 바인딩으로
//   TISCopyCurrentKeyboardInputSource / kTISPropertyInputSourceID 를 읽어
//   실제 한국어 입력 소스 활성 여부를 판단하면 상태 어긋남을 줄일 수 있다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use rdev::{grab, simulate, Event, EventType, Key};

/// 현재 IME 모드 (false=영문, true=한국어).
/// 전역 단일 값 — macOS 는 Windows 와 달리 IME 상태가 thread-local 이 아님.
static KOREAN_MODE: OnceLock<Mutex<bool>> = OnceLock::new();

/// ESC 신호를 워커 스레드로 전달하는 채널 송신단.
static SENDER: OnceLock<Sender<()>> = OnceLock::new();

/// simulate 로 보낸 Caps Lock 이 다시 callback 으로 들어올 때 무시하기 위한 플래그.
/// (Windows 의 LLKHF_INJECTED 대응)
static SENDING_CAPS: AtomicBool = AtomicBool::new(false);

fn korean_mode() -> &'static Mutex<bool> {
    KOREAN_MODE.get_or_init(|| Mutex::new(false))
}

/// 글로벌 키보드 후킹 콜백.
/// `Some(event)` 반환 → 이벤트 시스템으로 그대로 전달.
/// `None` 반환 → 이벤트 삼킴 (앱에 전달되지 않음).
fn keyboard_callback(event: Event) -> Option<Event> {
    match event.event_type {
        // 사용자가 직접 누른 Caps Lock → 한/영 모드 반전.
        // simulate 로 보낸 Caps Lock 은 SENDING_CAPS 로 구분해 건너뜀.
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
                // Caps Lock 시뮬레이션 직전에 상태를 false 로 먼저 갱신.
                // (simulate 가 callback 을 재진입하기 전에 상태를 확정)
                *mode = false;
                true
            } else {
                false
            }
        };

        if needs_send {
            // simulate 로 보낸 Caps Lock 은 callback 에서 무시하도록 플래그 설정.
            SENDING_CAPS.store(true, Ordering::SeqCst);

            // ESC 처리 직후보다 약간 늦게 전송해 이벤트 순서 안정화.
            thread::sleep(Duration::from_millis(10));
            let _ = simulate(&EventType::KeyPress(Key::CapsLock));
            thread::sleep(Duration::from_millis(10));
            let _ = simulate(&EventType::KeyRelease(Key::CapsLock));
            thread::sleep(Duration::from_millis(10));

            SENDING_CAPS.store(false, Ordering::SeqCst);
        }
    });

    // 글로벌 키보드 후킹 시작 (블로킹 — 메인 스레드 점유).
    // Accessibility 권한이 없으면 GrabError 반환.
    if let Err(err) = grab(keyboard_callback) {
        eprintln!("키보드 후킹 실패: {:?}", err);
        eprintln!(
            "시스템 환경설정 → 개인 정보 보호 및 보안 → 손쉬운 사용에서 이 앱을 허용해 주세요."
        );
        std::process::exit(1);
    }
}
