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
└── CLAUDE.md           # → AGENTS.md 심볼릭 링크
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

테스트 코드는 현재 없다. UI/시스템 후킹 동작이 핵심이라 수동 테스트 위주.

## 핵심 동작 흐름

1. `main` 에서 `WH_KEYBOARD_LL` 글로벌 키보드 후킹을 등록한다.
2. ESC 키다운 이벤트가 들어오면 `keyboard_proc` 가 워커 스레드로 채널 메시지를 보낸다.
3. `CallNextHookEx` 를 호출해 ESC 본래 동작은 그대로 통과시킨다 (가로채지 않음).
4. 워커 스레드는 짧은 시간(5ms) 동안 들어온 ESC 입력을 디바운스한 뒤 `force_english_ime` 호출.
5. `force_english_ime`:
   - `GetForegroundWindow` 로 현재 포커스된 창 핸들 획득
   - `ImmGetDefaultIMEWnd` 로 그 창의 IME 윈도우 핸들 획득
   - `WM_IME_CONTROL (0x0283)` + `IMC_SETOPENSTATUS (0x0006)` + `LPARAM(0)` 메시지를 `SendMessageTimeoutW` 로 송신 → IME 가 닫힘 (= 영문 모드)
6. 트레이 아이콘 좌/우 클릭 → 종료 메뉴 (`show_context_menu`).

## 주의사항

- `#![windows_subsystem = "windows"]` — 콘솔 창 없이 실행된다. `println!` 로는 디버깅이 불가능하다.
- 디버그 출력은 `%TEMP%\esc-eng-ime.log` 로 떨어지는 `dbg_log` 함수를 사용한다. 현재 무조건 기록 모드.
- 글로벌 키보드 후킹은 안티바이러스/EDR 에서 의심 행위로 분류될 수 있다. 배포 시 코드 사이닝 고려.
- IME 제어 상수(`WM_IME_CONTROL`, `IMC_SETOPENSTATUS`)는 windows-rs 바인딩에 노출되어 있지 않아 코드에 직접 정의되어 있다.
- 후킹 콜백은 OS 메시지 루프에서 동기적으로 실행되므로 무거운 작업은 워커 스레드로 위임해야 한다 (현재 구조 유지 필수).

## 환경변수

없음.

## 외부 의존 시스템

없음. Win32 API 만 사용.

## 의존성

- `windows = 0.58` — Win32 API 바인딩
  - 사용 features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_Input_Ime`, `Win32_UI_Shell`, `Win32_System_LibraryLoader`, `Win32_Graphics_Gdi`

## 보안 주의사항

- 글로벌 키보드 후킹은 모든 키 입력을 가로챌 수 있는 권한이라 보안적으로 민감한 영역이다.
- 현재 구현은 ESC 키만 식별해 IME 제어 신호를 보낼 뿐이며, 다른 키 입력 내용을 가공/저장하지 않는다.
- 디버그 로그에 키 입력 내용이나 식별 가능한 사용자 데이터를 남기지 않도록 주의한다.

## 개발 환경

- OS: Windows 10/11 (개발·테스트 모두 Windows 에서 진행)
- Rust edition 2021
- 64-bit 권장 (Win32 핸들 캐스팅 코드 가정)

## 커밋 규칙

- 커밋하기 전에 `README.md`, `AGENTS.md`, `PLAN.md` 를 검토한다.
- 코드 변경이 사용법·구조·계획에 영향을 준다면 같은 커밋에 문서 갱신을 포함한다.
- `PLAN.md` 는 진행 상황 마스터 문서이므로 의미 있는 변경마다 갱신한다.
- 비밀 키, 코드 사이닝 인증서, 개인 환경 의존 경로 등은 커밋하지 않는다.

## 병렬 작업 안내

이 저장소에서는 여러 에이전트가 동시에 작업할 수 있다. 자기 작업 영역 외 파일은 가급적 읽기만 하고, 다른 에이전트가 손대고 있는 영역은 변경하지 않는다.
