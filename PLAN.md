# PLAN.md

esc-eng-ime 개발 진행 상황 마스터 문서. 컨텍스트가 리셋되어도 이 파일 하나로 현재 상태와 다음 할 일을 파악할 수 있도록 유지한다. 의미 있는 변경마다 갱신.

## 현재 상태 (2026-05-08 기준)

핵심 동작이 thread별 자체 상태머신 기반으로 안정화되었다. 사용자 환경(Windows 11 Microsoft Korean IME)에서 메모장/터미널/브라우저를 오가며 동작 검증 완료.

### 완료된 항목

- ✅ 글로벌 키보드 후킹 (`WH_KEYBOARD_LL`)
- ✅ ESC 키다운 감지 후 IME 영문 모드 강제 전환
- ✅ ESC 본래 동작은 그대로 통과 (`CallNextHookEx`)
- ✅ 시스템 트레이 아이콘 상주 (`Shell_NotifyIconW`)
- ✅ 트레이 아이콘 좌/우 클릭 → 종료 메뉴 (`TrackPopupMenu`)
- ✅ 릴리스 빌드 최적화 설정 (`opt-level="z"`, `lto`, `strip`, `panic="abort"`)
- ✅ `#![windows_subsystem = "windows"]` — 콘솔 창 없이 실행
- ✅ 프로젝트 문서화 (README / AGENTS / PLAN)
- ✅ thread별 IME 상태 분리 추적 (창 전환 시 거꾸로 동작 문제 해결)

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

### M1. 전용 트레이 아이콘

현재 `LoadIconW(_, IDI_APPLICATION)` 으로 Windows 기본 응용프로그램 아이콘을 사용 중. 식별성이 떨어진다.

- [ ] `.ico` 리소스 파일 디자인 (간결한 한·영 전환 의미를 담은 작은 아이콘)
- [ ] `build.rs` + `embed-resource` (또는 `winres`) 크레이트로 리소스 임베드
- [ ] `LoadIconW` 호출에서 임베드된 리소스 ID 로 변경
- [ ] 라이트/다크 트레이 영역 모두에서 가시성 확인

### M2. 자동 시작 등록

Windows 부팅 시 자동 실행을 사용자 액션 없이 손쉽게 켤 수 있어야 한다.

- [ ] 등록 방식 결정 후보:
  - 시작 프로그램 폴더(`shell:startup`)에 바로가기 자동 생성
  - 레지스트리 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 항목 추가
- [ ] CLI 옵션 추가: `--install` / `--uninstall`, 또는 트레이 메뉴 "시작 시 자동 실행" 토글
- [ ] 등록 상태(현재 켜짐/꺼짐) 감지 및 표시
- [ ] README 의 자동 시작 섹션 갱신

## 향후 검토 항목 (백로그)

- 토글 키 화이트리스트 확장: VK_HANGUL 외 우측 Alt(VK_RMENU), Shift+Space 등 사용자가 쓸 수 있는 한/영 토글 키 옵션 노출
- 디버그 로그 옵션화: 환경변수(`ESC_ENG_IME_LOG=1`) 또는 빌드 피처로 켜고 끌 수 있게
- 단축키 커스터마이즈: ESC 외 다른 키 또는 조합 지원
- 한글 외 IME(일본어/중국어) 호환성 확인 — 현재는 한글 IME 가정
- ESC를 빈번히 쓰는 게임/풀스크린 앱과의 상호작용 검증
- 오류 발생 시 트레이 풍선 알림 / 자동 복구
- 코드 사이닝 (글로벌 키보드 후킹은 안티바이러스 오탐 가능성)
- 자동 업데이트 메커니즘
- `STATES` 의 종료된 thread entry 청소 로직

## 배포 방식

당분간 로컬 `cargo build --release` 후 `target\release\esc-eng-ime.exe` 직접 사용. GitHub Releases 등 자동 배포는 미정.

## 변경 이력

- 2026-05-07: 초기 문서 3종(`README.md`, `AGENTS.md`, `PLAN.md`) 작성. 핵심 동작은 이미 구현 완료된 상태.
- 2026-05-08: IME 제어 방식을 `WM_IME_CONTROL` 송신에서 **thread별 자체 상태 머신 + `SendInput(VK_HANGUL)` 토글** 로 전환. 창 전환 시 IME 상태 어긋남 문제 해결. 옛 디버그 로그/IME 상수 정리. Cargo features 정리.
- 2026-05-08: modern Microsoft Korean IME 환경에서 OS API 재검증 결과 기록.
  - `IMC_GETOPENSTATUS` 는 IME 활성/선택 여부만 반환 (한국어 IME 사용 중이면 항상 1) — conversion mode 와 무관하므로 결정 신호로 못 씀.
  - `IMC_SETOPENSTATUS` 는 응답은 하지만 화면상 mode 변경 없음.
  - `AttachThreadInput` + `ImmGetContext` 가 modern IME 환경에서도 NULL 반환 → `ImmGetConversionStatus` 사용 불가.
  - 결론: 자체 상태머신을 정식 채택. 마우스/`Win+Space` 토글은 추적 불가 한계로 받아들이고 한 번의 한/영키 토글로 동기화하는 사용 방식을 권장.
  - TSF API(`ITfThreadMgr`)나 `WH_CALLWNDPROC` 시스템 hook 으로 `WM_IME_NOTIFY` 가로채기는 백로그로 보류.
