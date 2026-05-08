# PLAN.md

eng-on-esc 개발 진행 상황 마스터 문서. 컨텍스트가 리셋되어도 이 파일 하나로 현재 상태와 다음 할 일을 파악할 수 있도록 유지한다. 의미 있는 변경마다 갱신.

## 현재 상태 (2026-05-08 기준)

Windows 버전: 핵심 동작이 thread별 자체 상태머신 기반으로 안정화. 트레이 아이콘(M1), 자동 시작 토글(M2) 완료. v0.3.0 릴리즈.

macOS 버전(M3): 메뉴바 아이콘(Step 2)·LaunchAgent 자동 시작(Step 3)·Accessibility 권한 다이얼로그·README/CHANGELOG 갱신은 완료. **단, 핵심 IME 영문 전환 동작이 사용자 환경에서 동작하지 않음 → 키보드 후킹 + IME 제어 부분을 TIS API 기반으로 재설계 (M3-v2)**. 트레이/자동시작/Accessibility 코드는 그대로 재사용. v0.4.0 릴리즈는 M3-v2 완료 후 발행.

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

### Windows (`windows/src/main.rs`)

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

### macOS (`macos/src/main.rs`)

| 구성요소 | 역할 |
| --- | --- |
| `keyboard_callback` | `rdev::grab` 콜백. ESC → 워커 채널, `CapsLock` (non-injected) → `KOREAN_MODE` 토글. |
| `KOREAN_MODE` | 전역 `Mutex<bool>` — macOS는 IME 상태가 전역. |
| `SENDING_CAPS` | `AtomicBool` — simulate 로 보낸 CapsLock 을 callback 에서 구분하기 위한 플래그. |
| `plist_path` / `autostart_*` | `~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist` 생성/삭제로 자동 시작 관리. |
| `make_icon` | 22×22 RGBA 원형 template 아이콘 생성 (`tray_icon::Icon::from_rgba`). |
| `main` (워커 스레드) | ESC 디바운스 → 한국어 모드이면 `simulate(CapsLock)`. |
| `main` (grab 스레드) | `grab(keyboard_callback)` 백그라운드 실행. |
| `main` (메인 스레드) | tray-icon 메뉴바 아이콘 구성 + `EventLoopExtMacOS::set_activation_policy(Accessory)` + `event_loop.run()`. |

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

### M3. macOS 포팅 (Step 1–3, Caps Lock 시뮬레이션 방식) ⚠️ 폐기

macOS 에서도 동일한 기능(ESC → 한국어 IME 영문 전환)을 제공한다.

#### 완료된 인프라 (M3-v2 에서 그대로 재사용)

- [x] `macos/` 독립 Rust 프로젝트, `Cargo.toml`/`Cargo.lock` 정착
- [x] GitHub Actions 워크플로 `build-macos` job (`macos-latest`, `working-directory: macos`)
- [x] **메뉴바 아이콘 (`tray-icon 0.19` + `tao 0.30`)** — `EventLoop::run` 메인 스레드 점유, `ActivationPolicy::Accessory` 로 Dock 아이콘 숨김, CheckMenuItem 체크 표시
- [x] **LaunchAgent 자동 시작 토글** — `~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist` 생성/삭제, 메뉴 토글 + 체크 표시
- [x] **Accessibility 권한 다이얼로그** — `AXIsProcessTrustedWithOptions(prompt=true)` 호출, 미승인 시 종료
- [x] README/AGENTS/CLAUDE/CHANGELOG macOS 섹션

#### 폐기 사유 (Caps Lock 시뮬레이션 방식)

`simulate(KeyPress(Key::CapsLock))` 으로 한국어 IME 영문 전환을 시도했으나 사용자 환경에서 동작하지 않음. 원인:

- macOS 의 한/영 토글 키는 사용자 설정에 따라 다름: Caps Lock(Sierra+ 옵션, 사용자가 켜야 함), 우측 Cmd, Ctrl+Space, Cmd+Space, 또는 IME 자체 토글(구름입력기의 `` ` `` 등). Caps Lock 시뮬레이션은 **사용자가 명시적으로 Caps Lock 을 한/영 토글로 매핑한 경우에만** 효과가 있다.
- `rdev::simulate` 의 Caps Lock 이벤트가 macOS 의 IME 시스템까지 전달되지 않거나, Caps Lock 자체 상태(대문자 lock)만 토글되고 IME 와 무관하게 끝나는 케이스 다수.
- 자체 상태 머신(KOREAN_MODE)은 OS IME 상태와 어긋나기 쉬움 — 첫 ESC 무동작, 마우스로 IME 토글 시 영구 어긋남 등 한계가 누적.

업계 표준은 **TIS(Text Input Sources) API 직접 호출** (macism, kawa, im-select, autokbisw, vim-im-select, modal-ime-switcher 등 모두 동일). 이 방향으로 재설계.

### M3-v2. macOS IME 제어 재설계 (TIS API 기반) 🚧 진행 예정

#### 핵심 변경

| 폐기 | 신규 |
| --- | --- |
| `simulate(Key::CapsLock)` 로 한/영 토글 | `TISSelectInputSource(<ABC>)` 로 영문 입력 소스 직접 선택 |
| `KOREAN_MODE: Mutex<bool>` 자체 상태 머신 | `TISCopyCurrentKeyboardInputSource()` 로 OS 상태 직접 조회 |
| `SENDING_CAPS` 재진입 가드 | 제거 (Caps Lock 키 자체를 안 건드림) |
| `rdev::grab` + `unstable_grab` 피처 | `rdev::listen` (이벤트 차단 없음, 더 가벼움/안정적) |
| `keyboard_callback` Caps Lock 추적 분기 | ESC keydown 한 줄만 보면 됨 |

#### 동작 흐름 (목표)

```
ESC keydown 감지 (rdev::listen 콜백)
  → 5ms 디바운스
  → src = TISCopyCurrentKeyboardInputSource()
  → src.id 가 "com.apple.inputmethod.Korean."로 시작
    OR  src.languages == ["ko", ...] 이면:
      → TISSelectInputSource(<캐시된 ABC source>)
  → 그 외엔 무동작
```

#### 작업 항목

**A. TIS API Rust 바인딩** (~50줄)
- [ ] `Carbon` framework link: `#[link(name = "Carbon", kind = "framework")]`
- [ ] extern "C" 선언: `TISCopyCurrentKeyboardInputSource`, `TISCreateInputSourceList`, `TISGetInputSourceProperty`, `TISSelectInputSource`
- [ ] CFString 키 상수: `kTISPropertyInputSourceID`, `kTISPropertyInputSourceLanguages` (extern static 또는 `CFString::new`)
- [ ] `core-foundation` crate 로 CFArray/CFDictionary/CFString 래핑

**B. `InputSourceManager` 모듈**
- [ ] `current_is_korean() -> bool` — 현재 입력 소스가 한국어인지 (prefix `com.apple.inputmethod.Korean.` OR languages 첫 원소 == "ko")
- [ ] `find_abc() -> Option<TISInputSourceRef>` — 시작 시 한 번 호출. 우선순위:
  1. `com.apple.keylayout.ABC`
  2. `com.apple.keylayout.US`
  3. 그 외 `com.apple.keylayout.*` 중 languages 가 "ko" 가 아닌 첫 번째
- [ ] `select_abc()` — 캐시된 ABC source 로 `TISSelectInputSource` 호출
- [ ] ABC source 캐시: `OnceLock<Option<RetainedRef>>` — 시작 시 한 번 검색 후 stderr 에 결과 한 줄 출력 (디버그 보조)

**C. `main.rs` 통합**
- [ ] `KOREAN_MODE`, `SENDING_CAPS`, `korean_mode()` 제거
- [ ] `keyboard_callback` 을 ESC KeyPress 한 분기만 남김
- [ ] `rdev::grab` → `rdev::listen` 으로 교체 (콜백 시그니처 변경 — 반환값 없음)
- [ ] 워커 스레드의 `simulate(Key::CapsLock)` 블록을 `if input_source::current_is_korean() { input_source::select_abc(); }` 로 교체
- [ ] `Cargo.toml` 에서 `rdev = { features = ["unstable_grab"] }` 의 features 제거

**D. 수동 테스트 매트릭스** (사용자 환경)
- [ ] Apple 2-Set Korean (`com.apple.inputmethod.Korean.2SetKorean`)
- [ ] Apple 3-Set Korean (`com.apple.inputmethod.Korean.3SetKorean390` 등) — 가능하면
- [ ] 구름입력기 (`com.gureum.*` 또는 별도 ID) — 설치돼 있다면
- [ ] 시작 시 한국어 모드 → 첫 ESC 가 즉시 영문 전환되는지 (구 방식의 "첫 ESC 무동작" 한계 해소 확인)
- [ ] 마우스로 메뉴바 IME 직접 변경 → ESC 가 정확히 동작하는지
- [ ] Cmd+Space (Spotlight 와 충돌하는 환경 회피) / Ctrl+Space 입력 소스 변경 후 ESC
- [ ] ESC 본래 동작이 그대로 통과하는지 (모달/Spotlight/IDE/브라우저 등)
- [ ] 멀티 디스플레이/스페이스 전환 후

**E. 문서/메뉴 정리**
- [ ] AGENTS/CLAUDE: macOS 핵심 동작 흐름을 TIS API 기반으로 다시 기술. "Caps Lock 추적", "SENDING_CAPS", "KOREAN_MODE", "첫 ESC 무동작" 한계 문구 제거.
- [ ] README 한계 섹션: Caps Lock 의존성 문구 제거. "키보드 설정에 ABC 또는 영문 keylayout 이 추가돼 있어야 함" 안내 추가.
- [ ] CHANGELOG: v0.4.0 항목을 미발행 상태에서 갱신 — Caps Lock 방식 → TIS 방식으로.
- [ ] PLAN: 변경 이력 추가, 본 섹션을 ✅ 완료로 마킹.

**F. 마무리 (v0.4.0 발행)**
- [ ] `macos/Cargo.toml` version 유지/검토
- [ ] 사용자 환경에서 `cargo build --release` + 수동 테스트 통과
- [ ] v0.4.0 태그 push → release.yml 가 macOS 바이너리 업로드

#### 기술 메모 (새 세션에서 참고)

- **TIS API 위치**: `Carbon.framework / HIToolbox.framework`. Apple 공식 문서상 deprecated 표시지만 macOS 15(Sequoia)까지 정상 동작. macism/kawa/im-select 모두 동일 API 사용 중.
- **CJKV 버그**: `TISSelectInputSource` 가 CJKV(Chinese/Japanese/Korean/Vietnamese) **로 들어갈 때** 가끔 실패. **CJKV 에서 빠질 때(우리 케이스: Korean → ABC)는 정상.** macism `InputSourceManager.swift` 의 `select()` 도 비-CJKV 타깃은 단순 `TISSelectInputSource(target)` 한 줄로 끝낸다.
- **`com.apple.keylayout.ABC` 가 사용자에게 없을 때**: 키보드 설정에 ABC/영문 레이아웃을 하나도 안 추가한 사용자가 있을 수 있음. fallback 후에도 못 찾으면 무동작 + 시작 시 stderr 경고(메뉴바 전용 앱이라 사용자가 직접 보긴 어려움 — README 안내가 1차 수단).
- **`rdev::listen` vs `grab`**: listen 은 이벤트를 관찰만, grab 은 차단 가능. 우리 앱은 ESC 를 차단하지 않으니 listen 으로 충분. listen 은 stable API 라 `unstable_grab` 피처 불필요.
- **CFString 키 상수**: `kTISPropertyInputSourceID` 같은 외부 상수는 Rust 에서 `extern "C" { static kTISPropertyInputSourceID: CFStringRef; }` 로 link 가능. 실패 시 `CFString::new("TISPropertyInputSourceID")` 로 동적 생성도 가능 (성능 차이 무시 가능).
- **레퍼런스 카운트**: `TISCopyCurrentKeyboardInputSource` 는 +1, `TISGetInputSourceProperty` 는 +0. core-foundation crate 의 `TCFType::wrap_under_create_rule` / `wrap_under_get_rule` 헬퍼 사용.
- **Gatekeeper / 메뉴바 전용 앱 / Accessibility** — 기존 정리 그대로 적용.

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
- 2026-05-08: v0.3.0 릴리즈 준비. v0.2.0 GitHub Release 바이너리에 M2 자동 실행 토글이 누락된 사실을 사용자가 발견. `Cargo.toml`/`Cargo.lock` version 0.2.0 → 0.3.0, `CHANGELOG.md` 에 [0.3.0] 항목 추가 (M2 자동 실행 토글 + README 사용자용 재구성). v0.3.0 태그 push → release.yml 이 자동 빌드/업로드.
- 2026-05-08: M3 시작. macOS 포팅을 위한 디렉토리 구조 재편. 기존 Windows 프로젝트를 `windows/` 로 이동(`git mv`), `macos/` 독립 Rust 프로젝트 신설. `rdev` 기반 글로벌 키보드 후킹 + 상태 머신 스켈레톤 작성. GitHub Actions 워크플로를 `build-windows` / `build-macos` / `publish` 3-job 구조로 변경. AGENTS.md/CLAUDE.md/PLAN.md 문서 갱신.
- 2026-05-08: M3 Step 1–3 완료. `rdev = { features = ["unstable_grab"] }` 피처 추가(Step 1). `tray-icon = "0.19"` + `tao = "0.30"` 추가, `grab` 을 백그라운드 스레드로 이동, tao 이벤트 루프를 메인 스레드에서 실행, `ActivationPolicy::Accessory` 로 Dock 아이콘 숨김(Step 2). LaunchAgent plist 기반 자동 시작 토글(`autostart_enable/disable/is_enabled`) + 메뉴 `CheckMenuItem` 체크 표시 연동(Step 3). `cargo check` / `cargo clippy` / `cargo build --release` 모두 통과.
- 2026-05-08: M3 Caps Lock 시뮬레이션 방식 폐기 결정 (사용자 환경에서 IME 영문 전환 미동작). 업계 표준(macism/kawa/im-select/autokbisw/vim-im-select 등) 조사 결과 모두 **TIS API 직접 호출** 방식 사용 확인. CJKV 버그는 CJKV 로 진입할 때만 발생하고 빠져나올 때(Korean→ABC, 우리 케이스)는 정상 동작이라는 점도 확인. M3-v2 로 재설계: `rdev::listen` + `TISCopyCurrentKeyboardInputSource` + `TISSelectInputSource(ABC)`. 자체 상태 머신/SENDING_CAPS 폐기. 트레이/자동시작/Accessibility 코드는 그대로 유지. 사용자 결정: Caps Lock 방식 완전 폐기, rdev 모드는 grab → listen 전환.
