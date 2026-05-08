# AGENTS.md

이 저장소에서 AI 에이전트가 작업할 때 알아야 할 정보.

## 프로젝트 개요

Rust로 작성한 상주 유틸리티. ESC 키를 누르면 한글 IME를 영문 모드로 강제 전환한다. ESC 본래 동작(취소/닫기 등)은 그대로 통과시킨다.

플랫폼별로 완전히 분리된 두 개의 독립 Rust 프로젝트로 구성된다.

## 프로젝트 구조

```
.
├── windows/            # Windows 전용 Rust 프로젝트
│   ├── Cargo.toml      # windows crate 0.58 + build-dep embed-resource
│   ├── Cargo.lock
│   ├── build.rs        # CARGO_PKG_VERSION 으로 ICON+VERSIONINFO 통합 .rc 를 OUT_DIR 에 생성
│   ├── src/
│   │   └── main.rs     # 키보드 후킹, 트레이 아이콘, IME 제어 (Win32 API)
│   ├── assets/
│   │   └── icon.ico    # 트레이 아이콘 (멀티 사이즈 ICO, exe 에 임베드)
│   └── tools/
│       └── generate-icon.ps1   # icon.ico 재생성 스크립트
├── macos/              # macOS 전용 Rust 프로젝트
│   ├── Cargo.toml      # rdev (CGEventTap 기반 글로벌 후킹)
│   └── src/
│       └── main.rs     # 키보드 후킹 + 상태 머신 (tray TODO)
├── .github/
│   └── workflows/
│       └── release.yml # v* 태그 → Windows/macOS 병렬 빌드 → GitHub Releases
├── README.md
├── CHANGELOG.md        # Keep a Changelog 양식
├── AGENTS.md           # 이 문서
├── PLAN.md             # 진행 상황 마스터 문서
└── CLAUDE.md           # → AGENTS.md 와 동일한 내용
```

## 빌드 / 실행 / 검증

### Windows (`windows/` 디렉토리에서)

```sh
# 디버그 빌드
cargo build

# 릴리스 빌드 (opt-level="z", lto, strip 적용)
cargo build --release

# 실행 (콘솔 없이 트레이 상주 모드)
cargo run --release

# 정적 분석
cargo check
cargo clippy
```

GNU 툴체인: `cargo +stable-x86_64-pc-windows-gnu build --release`

### macOS (`macos/` 디렉토리에서)

```sh
cargo build
cargo build --release
cargo run --release   # Accessibility 권한 필요 (손쉬운 사용)
cargo check
cargo clippy
```

테스트 코드는 없다. UI/시스템 후킹 동작이 핵심이라 수동 테스트 위주.

## 핵심 동작 흐름 (공통)

OS IME API 에 의존하지 않고 **자체 상태 머신**으로 IME 모드를 추적하는 방식은 두 플랫폼 공통이다.

- Windows: `VK_HANGUL` (한/영 키) 추적, 주입 이벤트는 `LLKHF_INJECTED` 로 구분
- macOS: `Caps Lock` 추적, 주입 이벤트는 `SENDING_CAPS` 플래그로 구분

## Windows 핵심 동작 흐름

OS의 IME API(`WM_IME_CONTROL/IMC_SETOPENSTATUS`, `VK_IME_OFF`, `ImmSetOpenStatus`, `GetKeyState(VK_HANGUL)` 토글 비트 등)는 Windows 11 modern Microsoft IME 환경에서 신뢰성 있는 응답을 주지 않는다는 사실이 확인되었다. 따라서 OS API에 의존하지 않고 **자체 상태 머신**으로 IME 모드를 추적한다.

### 상태 추적

- `STATES: HashMap<u32, bool>` — thread ID 별로 IME 모드(false=영문, true=한국어)를 보관. Windows IME 상태가 thread-local이므로 창마다 독립 추적이 필요하다.
- 새로 만나는 thread는 영문이라 가정하고 `false` 로 시작.
- `LL hook` 이 사용자가 직접(non-injected) 누른 `VK_HANGUL` 을 감지하면 `GetForegroundWindow → GetWindowThreadProcessId` 로 얻은 thread의 상태를 토글.
- ESC 처리 시 사용한 `send_vk(VK_HANGUL)` 은 INJECTED 플래그가 붙은 채 다시 LL hook 으로 들어오므로, hook 은 INJECTED 를 무시하고 ESC 처리 경로에서 직접 상태를 영문으로 set 한다 (이중 토글 방지).

### ESC 처리

1. `keyboard_proc` 에서 ESC 키다운 감지 → 워커 채널로 신호 송신.
2. `CallNextHookEx` 호출로 ESC 본래 동작은 그대로 통과 (가로채지 않음).
3. 워커가 5ms 디바운스 후 포그라운드 thread 의 상태 조회.
4. 상태가 한국어이면 `SendInput(VK_HANGUL)` 으로 한/영키 시뮬레이션 → IME 가 영문으로 전환됨. 동시에 우리 상태도 `false` 로 갱신.
5. 상태가 영문이면 아무 동작도 안 함 (부작용 없음).

### 한계

- 시작 시 한국어 모드인 thread의 첫 ESC 는 무동작. 한 번 한/영키 토글로 동기화된다.
- 마우스로 작업표시줄 IME 토글, `Win+Space` 입력 언어 변경 등은 추적되지 않는다 — 어긋나면 한 번 한/영키 토글로 복원.
- VK_RMENU(우측 Alt) 는 추적 대상 아님. 사용자가 우측 Alt 를 한/영 토글로 쓰지 않는 환경 기준. 다른 키로 토글한다면 LL hook 의 화이트리스트를 확장해야 한다.

### 트레이 아이콘

- `Shell_NotifyIconW(NIM_ADD/NIM_DELETE)` 로 아이콘 등록/해제.
- 좌/우 클릭 모두 `show_context_menu` 로 컨텍스트 메뉴(`TrackPopupMenu`) 표시.
- 메뉴 항목: ① 시작 시 자동 실행(`IDM_TOGGLE_AUTOSTART`, 등록 상태에 따라 `MF_CHECKED`/`MF_UNCHECKED`), ② 종료(`IDM_EXIT`).
- 아이콘은 `build.rs` 가 `CARGO_PKG_VERSION` 으로 ICON+VERSIONINFO 통합 `.rc` 를 OUT_DIR 에 자동 생성한 뒤 `embed-resource` 가 컴파일해 exe 에 임베드한다. 런타임에는 `LoadIconW(hinst, MAKEINTRESOURCE(IDI_TRAY_ICON=1))` 로 로드. 실패 시 `LoadIconW(IDI_APPLICATION)` 으로 폴백.
- 아이콘 디자인은 ESC 키 캡 모양. `tools/generate-icon.ps1` 로 재생성 가능 (System.Drawing). 16/24/32/48/64/256 PX 멀티 사이즈 PNG 를 ICO 컨테이너에 패킹.
- 아이콘을 수정한 뒤에는 `assets/icon.ico` 의 mtime 이 바뀌면 `build.rs` 가 다시 돌아 OUT_DIR 의 `.rc` 도 재생성된다 (`cargo:rerun-if-changed=assets/icon.ico` 로 추적).
- VERSIONINFO(파일 속성 메타데이터)도 같은 `.rc` 에 함께 들어간다. CompanyName/LegalCopyright 는 `kil9` / `Copyright (C) 2026 kil9`. `Cargo.toml` 의 `version` 만 갱신하면 FILEVERSION/ProductVersion 등이 자동으로 따라온다.

### 자동 시작 (HKCU\Run)

- 메뉴 토글이 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 의 `eng-on-esc` 값을 추가/삭제한다. 키 존재 여부 = 등록 상태.
- 등록 시 값에는 `GetModuleFileNameW` 로 얻은 현재 exe 의 절대 경로를 큰따옴표로 감싸 저장 (`"C:\path\to\eng-on-esc.exe"`). 경로에 공백이 있어도 안전.
- HKCU 라 관리자 권한 불필요. exe 를 옮긴 경우 사용자가 메뉴를 OFF→ON 다시 토글해 새 경로로 갱신.
- 등록/해제는 `RegOpenKeyExW(KEY_QUERY_VALUE | KEY_SET_VALUE)` + `RegSetValueExW(REG_SZ)` / `RegDeleteValueW`. Run 키 자체는 항상 존재한다고 가정.
- 함수: `autostart_is_enabled` / `autostart_enable` / `autostart_disable`. 메뉴는 `WM_COMMAND` → `IDM_TOGGLE_AUTOSTART` 분기에서 호출.

## macOS 핵심 동작 흐름

### 상태 추적

- `KOREAN_MODE: Mutex<bool>` — 전역 단일 값 (macOS 는 IME 상태가 thread-local 이 아님).
- `rdev::grab` 콜백에서 사용자가 직접 누른 `Caps Lock` 을 감지 → 상태 반전.
- `SENDING_CAPS: AtomicBool` — simulate 로 보낸 Caps Lock 이 callback 으로 재진입할 때 무시하기 위한 플래그 (Windows 의 `LLKHF_INJECTED` 대응).

### ESC 처리

1. `keyboard_callback` 에서 ESC KeyPress 감지 → 워커 채널로 신호 송신.
2. ESC 이벤트는 `Some(event)` 로 통과.
3. 워커가 5ms 디바운스 후 `KOREAN_MODE` 확인.
4. 한국어 모드이면 `KOREAN_MODE = false` 로 set, `simulate(KeyPress(CapsLock))` 전송.
5. 영문 모드이면 아무 동작 없음.

### 한계

- 시작 시 한국어 모드인 경우 첫 ESC 는 무동작. 한 번 Caps Lock 으로 동기화.
- 마우스/입력 소스 직접 변경 등은 추적되지 않는다.
- Caps Lock 을 일반 대소문자 토글로 사용하는 환경(한국어 IME 미설치 Mac)에는 맞지 않는다.

### 트레이 (TODO)

- `tray-icon` + `tao` 로 `NSStatusItem` 메뉴바 상주 예정.
- tao 이벤트 루프가 메인 스레드를 점유해야 하므로 `grab` 은 별도 스레드로 이동 필요.
- 자동 시작: `~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist` 생성/삭제 예정.

## 주의사항

### Windows

- `#![windows_subsystem = "windows"]` — 콘솔 창 없이 실행된다. `println!` 로는 디버깅이 불가능하다.
- 디버깅이 필요하면 `dbg_log` 헬퍼를 임시로 추가해 `%TEMP%\eng-on-esc.log` 같은 경로로 append 하면 된다.
- 후킹 콜백은 OS 메시지 루프에서 동기적으로 실행되므로 무거운 작업은 워커 스레드로 위임해야 한다. LL hook 안에서 `Mutex::lock` 을 짧게 잡는 것은 허용되지만 길어지면 시스템 입력이 느려진다.
- `STATES` 의 키는 thread ID 라 thread 가 종료되어도 entry 가 그대로 남는다. 장기 실행 시 메모리 누수 가능 — 청소 로직 미구현.

### macOS

- `rdev::grab` 은 Accessibility(손쉬운 사용) 권한 없이는 실행되지 않는다.
- `grab` 은 호출 스레드를 블로킹하며 내부적으로 CGEventTap + CFRunLoop 를 실행한다.
- 트레이 구현 시 tao 이벤트 루프는 반드시 메인 스레드에서 실행해야 한다.

### 공통

- 글로벌 키보드 후킹은 안티바이러스/EDR 에서 의심 행위로 분류될 수 있다. 배포 시 코드 사이닝 고려.
- 디버그 로그를 추가할 때는 키 입력 내용이나 식별 가능한 사용자 데이터를 남기지 않도록 주의한다.

## 환경변수

없음.

## 외부 의존 시스템

없음.

## 의존성

### Windows (`windows/Cargo.toml`)

- `windows = 0.58` — Win32 API 바인딩
  - 사용 features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_Shell`, `Win32_System_LibraryLoader`, `Win32_System_Registry`, `Win32_Graphics_Gdi`
- `embed-resource = 2` (build-dependency) — `build.rs` 가 OUT_DIR 에 자동 생성한 `.rc` (ICON + VERSIONINFO) 를 컴파일해 exe 에 임베드.

### macOS (`macos/Cargo.toml`)

- `rdev = 0.5` — 크로스 플랫폼 글로벌 키보드 후킹. macOS 에서는 CGEventTap 기반.

## 개발 환경

- Windows: Windows 10/11, Rust edition 2021, 64-bit, MSVC 또는 GNU 툴체인
- macOS: macOS 12+, Rust edition 2021, Apple Silicon / Intel 모두 지원 예정

## 커밋 규칙

- 커밋하기 전에 `README.md`, `AGENTS.md`, `PLAN.md` 를 검토한다.
- 코드 변경이 사용법·구조·계획에 영향을 준다면 같은 커밋에 문서 갱신을 포함한다.
- `PLAN.md` 는 진행 상황 마스터 문서이므로 의미 있는 변경마다 갱신한다.
- 비밀 키, 코드 사이닝 인증서, 개인 환경 의존 경로 등은 커밋하지 않는다.

## 병렬 작업 안내

이 저장소에서는 여러 에이전트가 동시에 작업할 수 있다. 자기 작업 영역 외 파일은 가급적 읽기만 하고, 다른 에이전트가 손대고 있는 영역은 변경하지 않는다.
