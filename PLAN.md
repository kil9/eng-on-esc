# PLAN.md

eng-on-esc 개발 진행 상황 마스터 문서. 컨텍스트가 리셋되어도 이 파일 하나로 현재 상태와 다음 할 일을 파악할 수 있도록 유지한다. 의미 있는 변경마다 갱신.

## 현재 상태 (2026-05-08 기준)

핵심 동작이 thread별 자체 상태머신 기반으로 안정화되었다. 사용자 환경(Windows 11 Microsoft Korean IME)에서 메모장/터미널/브라우저를 오가며 동작 검증 완료. 전용 트레이 아이콘 도입(M1) 완료. M2 자동 시작 토글까지 구현 완료.

### 완료된 항목

- ✅ 글로벌 키보드 후킹 (`WH_KEYBOARD_LL`)
- ✅ ESC 키다운 감지 후 IME 영문 모드 강제 전환
- ✅ ESC 본래 동작은 그대로 통과 (`CallNextHookEx`)
- ✅ 시스템 트레이 아이콘 상주 (`Shell_NotifyIconW`)
- ✅ 트레이 아이콘 좌/우 클릭 → 컨텍스트 메뉴 (`TrackPopupMenu`)
- ✅ 릴리스 빌드 최적화 설정 (`opt-level="z"`, `lto`, `strip`, `panic="abort"`)
- ✅ `#![windows_subsystem = "windows"]` — 콘솔 창 없이 실행
- ✅ 프로젝트 문서화 (README / AGENTS / PLAN)
- ✅ thread별 IME 상태 분리 추적 (창 전환 시 거꾸로 동작 문제 해결)
- ✅ M1 전용 트레이 아이콘 (ESC 키 캡 모양, 멀티 사이즈 ICO, `build.rs` + `embed-resource` 로 exe 에 임베드)
- ✅ M2 자동 시작 토글 (트레이 메뉴 → `HKCU\...\Run\eng-on-esc` 등록/해제, 체크 표시로 상태 노출)

### IME 제어 방식 결정 경위

다음 방법들을 차례로 시도했으나 Windows 11 Microsoft Korean IME(modern, TSF 기반) 환경에서 신뢰성 있게 동작하지 않음:

1. `WM_IME_CONTROL + IMC_SETOPENSTATUS` 송신 — `SendMessageTimeoutW` 가 성공을 반환하지만 IME 모드는 변경되지 않음.
2. `AttachThreadInput` 후 `ImmGetContext + ImmSetOpenStatus` — `ImmGetContext` 가 NULL 반환.
3. `SendInput(VK_IME_OFF)` — 효과 없음.
4. `GetKeyState(VK_HANGUL)` 토글 비트로 모드 조회 → 조건부 `SendInput(VK_HANGUL)` — 토글 비트가 사용자 실제 IME 상태와 동기화되지 않아 부작용 발생.

최종적으로 **자체 상태 머신** 채택: LL hook이 사용자가 직접 누른 `VK_HANGUL` 을 추적해 thread별 모드를 보관, ESC 시 그 상태가 한국어이면 `SendInput(VK_HANGUL)` 으로 영문 토글. 이 방식이 IME 종류와 무관하게 가장 안정적이지만, 마우스/`Win+Space` 등 키보드 외 경로 변경은 추적 못 한다는 한계가 있다.

## 코드 구조 요약

`src/main.rs` 단일 파일.

| 구성요소 | 역할 |
| --- | --- |
| `keyboard_proc` | 글로벌 후킹 콜백. ESC → 워커 채널, `VK_HANGUL` (non-injected) → 해당 thread 의 IME 상태 토글. |
| `send_vk` | `SendInput` 으로 가상 키 down/up 시뮬레이션. |
| `wnd_proc` | 트레이 메시지/메뉴 명령 처리. |
| `add_tray` / `remove_tray` | 트레이 아이콘 등록/해제. |
| `show_context_menu` | 종료 메뉴 표시. |
| `states` | 전역 thread→IME상태 맵 (`Mutex<HashMap<u32, bool>>`) lazy 초기화. |
| `foreground_tid` | `GetForegroundWindow` + `GetWindowThreadProcessId` 헬퍼. |
| `main` | 후킹 등록, 윈도우/트레이 생성, 메시지 루프 실행. 워커 스레드 생성. |

상수:
- `LLKHF_INJECTED = 0x10`
- `WM_TRAYICON = WM_USER + 1`
- `IDM_EXIT = 1001`
- `TRAY_UID = 1`

## 다음 마일스톤 (우선순위 순)

### M0. 첫 릴리즈 v0.1.0 ✅ 완료

GitHub Releases 로 단일 exe 를 배포해 사용자가 직접 빌드 없이 받아 쓸 수 있게 한다. 첫 릴리즈는 코드 사이닝 없이 unsigned exe 로 진행하고, 파일 메타데이터(VERSIONINFO)만 임베드해 식별성을 확보한다.

- [x] `VERSIONINFO` 블록 추가 — 제품명/파일 설명/회사·저작권/파일 버전·제품 버전. `build.rs` 가 OUT_DIR 에 ICON+VERSIONINFO 통합 `.rc` 를 생성해 `embed-resource` 로 임베드. 기존 `assets/icon.rc` 는 삭제.
- [x] `Cargo.toml` 의 `version` 과 `VERSIONINFO` 버전 동기화 — `build.rs` 에서 `CARGO_PKG_VERSION` 을 읽어 자동 반영 (디자인 결정 참조).
- [x] `CHANGELOG.md` 신설, v0.1.0 항목 작성 (Keep a Changelog 양식).
- [x] `.github/workflows/release.yml` 작성:
  - 트리거: `v*` 태그 push (예: `v0.1.0`)
  - runner: `windows-latest` (MSVC 툴체인)
  - 단계: `dtolnay/rust-toolchain@stable` → `Swatinem/rust-cache@v2` → `cargo build --release --locked` → `Get-FileHash` 로 `SHA256SUMS.txt` 생성 → `softprops/action-gh-release@v2` 로 업로드 (`generate_release_notes: true`)
- [x] `README.md` 에 다운로드/설치 섹션 추가 — Releases 링크, SmartScreen 안내, `Get-FileHash` 검증 명령 포함.
- [x] v0.1.0 태그 발행 및 워크플로 동작 확인. https://github.com/kil9/eng-on-esc/releases/tag/v0.1.0 — `eng-on-esc.exe` (162,304 B) + `SHA256SUMS.txt` 자동 업로드. 다운로드 후 SHA256 일치 및 VERSIONINFO 정상 노출 확인 (`ProductName=eng-on-esc`, `FileVersion=0.1.0.0`, `CompanyName=kil9`).

### M2. 자동 시작 등록 ✅ 완료

Windows 부팅 시 자동 실행을 사용자 액션 없이 손쉽게 켤 수 있게 한다.

- [x] 등록 방식: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 의 `eng-on-esc` 값 추가/삭제 (디자인 결정 참조). 시작 프로그램 폴더 + 바로가기 방식은 `IShellLinkW` COM 호출이 필요해 코드량이 크고, 사용자 검증 결과도 동일해 채택하지 않음.
- [x] UI: 트레이 메뉴 토글 — "시작 시 자동 실행" 항목 클릭 시 등록/해제. 별도 CLI 옵션은 추가하지 않음 (필요해지면 백로그에서 검토).
- [x] 등록 상태 표시: 메뉴 항목 앞 체크 표시 (`MF_CHECKED`/`MF_UNCHECKED`). `autostart_is_enabled()` 가 키 존재 여부로 판정.
- [x] README 의 자동 시작 섹션 갱신.

## 향후 검토 항목 (백로그)

- 토글 키 화이트리스트 확장: VK_HANGUL 외 우측 Alt(VK_RMENU), Shift+Space 등 사용자가 쓸 수 있는 한/영 토글 키 옵션 노출
- 디버그 로그 옵션화: 환경변수(`ENG_ON_ESC_LOG=1`) 또는 빌드 피처로 켜고 끌 수 있게
- 단축키 커스터마이즈: ESC 외 다른 키 또는 조합 지원
- 한글 외 IME(일본어/중국어) 호환성 확인 — 현재는 한글 IME 가정
- ESC를 빈번히 쓰는 게임/풀스크린 앱과의 상호작용 검증
- 오류 발생 시 트레이 풍선 알림 / 자동 복구
- 코드 사이닝 (글로벌 키보드 후킹은 안티바이러스 오탐 가능성)
- 자동 업데이트 메커니즘
- `STATES` 의 종료된 thread entry 청소 로직
- 릴리즈 워크플로 `actions/checkout` 과 `softprops/action-gh-release` 가 Node.js 20 기반 — 2026-09-16 이후 Node.js 24 강제. 그전에 액션 버전 업그레이드 또는 `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` 검토. `windows-latest` 가 `windows-2025` 로 redirect 되는 이슈도 같이 추적.

## 배포 방식

- 현재(릴리즈 전): 로컬 `cargo build --release` 후 `target\release\eng-on-esc.exe` 직접 사용.
- 목표(M0): GitHub Releases 에 GitHub Actions 워크플로가 `v*` 태그 push 트리거로 빌드해 unsigned exe + SHA256 체크섬을 업로드. 사용자는 Releases 페이지에서 받아 단일 exe 실행.
- 코드 사이닝은 비용·인증서 도입 비용 고려해 첫 릴리즈에서는 보류 (백로그). SmartScreen 경고는 README 에 안내.

### 디자인 결정: M0 버전 동기화 방식
- **날짜**: 2026-05-08
- **결정**: `build.rs` 에서 `CARGO_PKG_VERSION` 을 읽어 OUT_DIR 에 ICON 정의 + VERSIONINFO 블록을 통합한 `.rc` 를 생성하고 `embed-resource` 로 컴파일한다. `assets/icon.rc` 는 삭제하고 `assets/icon.ico` 만 유지. `build.rs` 가 `assets/icon.ico` 를 OUT_DIR 로 복사해 같은 디렉토리에서 참조하므로 rc.exe/windres 의 include 경로 의존을 회피.
- **이유**: Cargo.toml 의 `version` 만 바꾸면 exe 의 파일 속성과 Releases 자산 모두 자동 동기화. 한 번 셋업해 두면 이후 릴리즈마다 수동 동기화 실수가 없다.
- **대안**:
  - 수동 동기화 — 간단하지만 릴리즈마다 두 곳을 같이 갱신해야 함. 실수 가능.
  - `cargo-edit` / 외부 도구로 동기화 — 추가 의존성, CI 환경에서 셋업 부담.

### 디자인 결정: M2 자동 시작 등록 방식
- **날짜**: 2026-05-08
- **결정**: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 에 `eng-on-esc` 값으로 현재 exe 의 절대 경로(따옴표 감쌈)를 저장한다. UI 는 트레이 메뉴 토글로만 노출하고, 메뉴 항목 앞 체크 표시(`MF_CHECKED`)로 등록 상태를 표시한다. CLI 옵션은 도입하지 않는다. exe 위치가 바뀌면 사용자가 메뉴 OFF→ON 으로 갱신.
- **이유**: 단일 exe 단독 실행이 전제이고, HKCU\Run 은 표준 자동 실행 경로라 작업 관리자 시작프로그램 탭에도 표시된다. `RegOpenKeyExW` + `RegSetValueExW` 만으로 끝나 코드량이 적다. 트레이 메뉴 토글은 GUI 사용자에게 발견성이 가장 높고 현재 상태가 한눈에 보인다. CLI 옵션은 `#![windows_subsystem = "windows"]` 환경에서 콘솔 출력이 까다로워 비용 대비 효용이 작다.
- **대안**:
  - 시작 프로그램 폴더에 `.lnk` 바로가기 생성 — `IShellLinkW`/`IPersistFile` COM 호출이 필요해 코드량이 두 배. 사용자가 폴더에서 직접 확인할 수 있다는 장점은 있으나, 작업 관리자 시작프로그램 탭에도 동일하게 노출돼 식별성 차이는 작다.
  - CLI 옵션 (`--install`/`--uninstall`) 추가 — 스크립팅에는 유리하나 GUI 우선이라 보류. 필요해지면 메뉴 토글과 동일 함수를 호출하는 진입점만 추가하면 된다.
  - `RegCreateKeyExW` 로 키 생성까지 시도 — `Win32_Security` feature 가 추가로 필요. Run 키는 항상 존재하므로 `RegOpenKeyExW` 만으로 충분.

### 디자인 결정: M1 아이콘 임베드 방식
- **날짜**: 2026-05-08
- **결정**: `build.rs` + `embed-resource = 2` (build-dependency) 로 `assets/icon.rc` 를 컴파일해 exe 에 ICON 리소스를 임베드한다. `main.rs` 는 `LoadIconW(hinst, MAKEINTRESOURCE(1))` 로 로드, 실패 시 `IDI_APPLICATION` 폴백.
- **이유**: 표준적인 Windows 리소스 임베드 방식이고 단일 exe 로 배포 가능. `tools/generate-icon.ps1` 로 디자인은 분리 관리, 빌드 시 자동 재컴파일.
- **대안**:
  - 런타임 `LoadImageW(LR_LOADFROMFILE)` 로 외부 `icon.ico` 로딩 — exe 옆 별도 파일 필요(배포 번거로움). 처음 한 차례 시도했으나 사용자 빌드 환경(VS) 복구 후 임베드 방식으로 환원.
  - `include_bytes!` + `CreateIconFromResourceEx` 로 메모리 내 ICO 파싱 — `RT_GROUP_ICON` 포맷을 수동으로 다루어야 해 복잡.

## 변경 이력

- 2026-05-07: 초기 문서 3종(`README.md`, `AGENTS.md`, `PLAN.md`) 작성. 핵심 동작은 이미 구현 완료된 상태.
- 2026-05-08: IME 제어 방식을 `WM_IME_CONTROL` 송신에서 **thread별 자체 상태 머신 + `SendInput(VK_HANGUL)` 토글** 로 전환. 창 전환 시 IME 상태 어긋남 문제 해결. 옛 디버그 로그/IME 상수 정리. Cargo features 정리.
- 2026-05-08: modern Microsoft Korean IME 환경에서 OS API 재검증 결과 기록.
  - `IMC_GETOPENSTATUS` 는 IME 활성/선택 여부만 반환 (한국어 IME 사용 중이면 항상 1) — conversion mode 와 무관하므로 결정 신호로 못 씀.
  - `IMC_SETOPENSTATUS` 는 응답은 하지만 화면상 mode 변경 없음.
  - `AttachThreadInput` + `ImmGetContext` 가 modern IME 환경에서도 NULL 반환 → `ImmGetConversionStatus` 사용 불가.
  - 결론: 자체 상태머신을 정식 채택. 마우스/`Win+Space` 토글은 추적 불가 한계로 받아들이고 한 번의 한/영키 토글로 동기화하는 사용 방식을 권장.
  - TSF API(`ITfThreadMgr`)나 `WH_CALLWNDPROC` 시스템 hook 으로 `WM_IME_NOTIFY` 가로채기는 백로그로 보류.
- 2026-05-08: M1 완료. `tools/generate-icon.ps1` 로 ESC 키 캡 모양 멀티 사이즈 `assets/icon.ico` 생성, `assets/icon.rc` + `build.rs` + `embed-resource` 로 exe 에 ICON 리소스 임베드. `main.rs` 의 트레이 아이콘 로딩을 `LoadIconW(IDI_APPLICATION)` 에서 `LoadIconW(hinst, MAKEINTRESOURCE(IDI_TRAY_ICON))` 로 변경, 실패 시 시스템 기본 아이콘 폴백 유지. `cargo build --release` 통과(MSVC), `cargo clippy` 경고 없음, exe 에 32×32 아이콘 추출 확인. (중간에 환경 문제로 런타임 `LoadImageW(LR_LOADFROMFILE)` 방식으로 잠시 갔다가, VS Build Tools 정상화 후 표준 임베드 방식으로 환원.)
- 2026-05-08: M0 구현 완료(태그 발행 전). `build.rs` 가 `CARGO_PKG_VERSION` 으로부터 ICON+VERSIONINFO 통합 `.rc` 를 OUT_DIR 에 자동 생성하도록 변경, `assets/icon.rc` 삭제. `CHANGELOG.md` 추가(Keep a Changelog 1.1.0 양식). `.github/workflows/release.yml` 추가 — `v*` 태그 push 시 windows-latest 에서 release 빌드 → `SHA256SUMS.txt` 생성 → `softprops/action-gh-release@v2` 로 unsigned exe 업로드. `README.md` 에 다운로드/SmartScreen/SHA256 검증 안내 추가.
- 2026-05-08: v0.1.0 태그 push (`origin/main` 1636041). Release 워크플로 1m16s 만에 성공, https://github.com/kil9/eng-on-esc/releases/tag/v0.1.0 에 `eng-on-esc.exe` + `SHA256SUMS.txt` 정식 publish (draft=false). 다운로드한 exe 의 SHA256 매치, VERSIONINFO 정상 노출 확인. 워크플로 deprecation 경고(actions/checkout@v4, softprops/action-gh-release@v2 의 Node.js 20, windows-2025 마이그레이션) 백로그에 등록.
- 2026-05-08: 프로젝트 이름을 `esc-eng-ime` → `eng-on-esc` 로 변경. Cargo 패키지명, 윈도우 클래스명, VERSIONINFO 메타데이터, GitHub Actions 워크플로 산출물명, 문서 전반 갱신. v0.2.0 으로 릴리즈 (Cargo.toml version 0.2.0, CHANGELOG.md 항목 추가). v0.1.0 GitHub Release 자산은 옛 이름으로 보존, v0.2.0 부터 새 이름 자산 발행.
- 2026-05-08: M2 완료. 트레이 메뉴에 "시작 시 자동 실행(&S)" 항목 추가, `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\eng-on-esc` 값 등록/해제 토글 구현. 등록 상태는 메뉴 항목 앞 체크 표시로 노출. `Cargo.toml` 에 `Win32_System_Registry` feature 추가. `cargo build --release` / `cargo clippy` 통과, 사용자 환경(`D:\work\eng-on-esc\target\release\eng-on-esc.exe`)에서 토글 → `reg query` 로 값 추가/삭제 직접 확인 완료.
