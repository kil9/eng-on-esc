# AGENTS.md

이 저장소에서 AI 에이전트가 작업할 때 알아야 할 정보.

## 프로젝트 개요

Rust로 작성한 Windows 전용 상주 유틸리티. ESC 키를 누르면 한글 IME를 영문 모드로 강제 전환한다. ESC 본래 동작(취소/닫기 등)은 그대로 통과시킨다.

## 프로젝트 구조

```
.
├── Cargo.toml          # Rust 프로젝트 정의 (windows crate 0.58 사용)
├── Cargo.lock
├── src/
│   └── main.rs         # 단일 엔트리포인트 — 키보드 후킹, 트레이 아이콘, IME 제어 모두 여기
├── target/             # 빌드 산출물 (커밋 제외)
├── README.md
├── AGENTS.md           # 이 문서
├── PLAN.md             # 진행 상황 마스터 문서
└── CLAUDE.md           # → AGENTS.md 와 동일한 내용
```

## 빌드 / 실행 / 검증

```sh
# 디버그 빌드
cargo build

# 릴리스 빌드 (사용 권장 — opt-level="z", lto, strip 적용)
cargo build --release

# 실행 (콘솔 없이 트레이 상주 모드로 시작)
cargo run --release

# 정적 분석
cargo check
cargo clippy
```

GNU 툴체인 환경에서 빌드하려면 `cargo +stable-x86_64-pc-windows-gnu build --release` 사용.

테스트 코드는 현재 없다. UI/시스템 후킹 동작이 핵심이라 수동 테스트 위주.

## 핵심 동작 흐름

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
- 좌/우 클릭 모두 `show_context_menu` 로 종료 메뉴(`TrackPopupMenu`).
- 아이콘은 현재 `IDI_APPLICATION` (Windows 기본). 전용 아이콘 임베드는 미정 (PLAN.md M1).

## 주의사항

- `#![windows_subsystem = "windows"]` — 콘솔 창 없이 실행된다. `println!` 로는 디버깅이 불가능하다.
- 디버깅이 필요하면 `dbg_log` 헬퍼를 임시로 추가해 `%TEMP%\esc-eng-ime.log` 같은 경로로 append 하면 된다 (현재 코드에는 포함되어 있지 않음).
- 글로벌 키보드 후킹은 안티바이러스/EDR 에서 의심 행위로 분류될 수 있다. 배포 시 코드 사이닝 고려.
- 후킹 콜백은 OS 메시지 루프에서 동기적으로 실행되므로 무거운 작업은 워커 스레드로 위임해야 한다 (현재 구조 유지 필수). LL hook 안에서 `Mutex::lock` 을 짧게 잡는 것은 허용되지만 길어지면 시스템 입력이 느려진다.
- `STATES` 의 키는 thread ID 라 thread 가 종료되어도 entry 가 그대로 남는다. 장기 실행 시 메모리 누수 가능 — 청소 로직은 미구현.

## 환경변수

없음.

## 외부 의존 시스템

없음. Win32 API 만 사용.

## 의존성

- `windows = 0.58` — Win32 API 바인딩
  - 사용 features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_Shell`, `Win32_System_LibraryLoader`, `Win32_Graphics_Gdi`

## 보안 주의사항

- 글로벌 키보드 후킹은 모든 키 입력을 가로챌 수 있는 권한이라 보안적으로 민감한 영역이다.
- 현재 구현은 ESC 와 `VK_HANGUL` 만 식별해 IME 제어 신호를 보낼 뿐이며, 다른 키 입력 내용을 가공/저장하지 않는다.
- 디버그 로그를 추가할 때는 키 입력 내용이나 식별 가능한 사용자 데이터를 남기지 않도록 주의한다.

## 개발 환경

- OS: Windows 10/11 (개발·테스트 모두 Windows 에서 진행)
- Rust edition 2021
- 64-bit 권장 (Win32 핸들 캐스팅 코드 가정)
- MSVC 또는 GNU 툴체인 모두 사용 가능. MSVC 사용 시 Visual Studio Build Tools 필요, GNU 사용 시 rustup 이 mingw-w64 를 함께 설치한다.

## 커밋 규칙

- 커밋하기 전에 `README.md`, `AGENTS.md`, `PLAN.md` 를 검토한다.
- 코드 변경이 사용법·구조·계획에 영향을 준다면 같은 커밋에 문서 갱신을 포함한다.
- `PLAN.md` 는 진행 상황 마스터 문서이므로 의미 있는 변경마다 갱신한다.
- 비밀 키, 코드 사이닝 인증서, 개인 환경 의존 경로 등은 커밋하지 않는다.

## 병렬 작업 안내

이 저장소에서는 여러 에이전트가 동시에 작업할 수 있다. 자기 작업 영역 외 파일은 가급적 읽기만 하고, 다른 에이전트가 손대고 있는 영역은 변경하지 않는다.
