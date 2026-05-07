# PLAN.md

esc-eng-ime 개발 진행 상황 마스터 문서. 컨텍스트가 리셋되어도 이 파일 하나로 현재 상태와 다음 할 일을 파악할 수 있도록 유지한다. 의미 있는 변경마다 갱신.

## 현재 상태 (2026-05-07 기준)

핵심 동작은 구현 완료된 상태. 현재 다른 에이전트가 **버그 수정** 작업을 병렬로 진행 중이므로 이 문서를 작성하는 동안 `src/` 내 파일은 읽기 전용으로 다룬다.

### 완료된 항목

- ✅ 글로벌 키보드 후킹 (`WH_KEYBOARD_LL`)
- ✅ ESC 키다운 감지 후 IME 영문 모드 강제 전환 (`WM_IME_CONTROL` + `IMC_SETOPENSTATUS=0`)
- ✅ ESC 본래 동작은 그대로 통과 (`CallNextHookEx`)
- ✅ 시스템 트레이 아이콘 상주 (`Shell_NotifyIconW`)
- ✅ 트레이 아이콘 좌/우 클릭 → 종료 메뉴 (`TrackPopupMenu`)
- ✅ 디버그 로그 기록 (`%TEMP%\esc-eng-ime.log`)
- ✅ 릴리스 빌드 최적화 설정 (`opt-level="z"`, `lto`, `strip`, `panic="abort"`)
- ✅ `#![windows_subsystem = "windows"]` — 콘솔 창 없이 실행
- ✅ 프로젝트 문서화 (README / AGENTS / PLAN)

### 진행 중

- 🔄 버그 수정 (다른 에이전트 담당, 세부 항목은 해당 에이전트 작업 결과로 확인)

## 코드 구조 요약

`src/main.rs` 단일 파일, 약 220 줄.

| 구성요소 | 역할 |
| --- | --- |
| `keyboard_proc` | 글로벌 후킹 콜백. ESC 키다운 시 워커로 채널 신호 송신. |
| `force_english_ime` | 포그라운드 창의 IME를 영문 모드로 강제 전환. |
| `wnd_proc` | 트레이 메시지/메뉴 명령 처리. |
| `add_tray` / `remove_tray` | 트레이 아이콘 등록/해제. |
| `show_context_menu` | 종료 메뉴 표시. |
| `dbg_log` | 디버그 로그를 `%TEMP%\esc-eng-ime.log` 에 append. |
| `main` | 후킹 등록, 윈도우/트레이 생성, 메시지 루프 실행. |

상수:
- `WM_IME_CONTROL = 0x0283`
- `IMC_SETOPENSTATUS = 0x0006`
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

### M2. 디버그 로그 토글

현재 `dbg_log` 가 무조건 `%TEMP%\esc-eng-ime.log` 에 기록한다. 운영 환경에서는 불필요한 디스크 쓰기.

- [ ] 토글 방식 결정: 환경변수(`ESC_ENG_IME_LOG=1`) vs 빌드 피처(`--features log`) vs 트레이 메뉴 토글
- [ ] 기본값은 비활성화
- [ ] `dbg_log` 진입부에서 조건 체크하도록 수정
- [ ] README 에 켜는 방법 문서화

### M3. 자동 시작 등록

Windows 부팅 시 자동 실행을 사용자 액션 없이 손쉽게 켤 수 있어야 한다.

- [ ] 등록 방식 결정 후보:
  - 시작 프로그램 폴더(`shell:startup`)에 바로가기 자동 생성
  - 레지스트리 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 항목 추가
- [ ] CLI 옵션 추가: `--install` / `--uninstall`, 또는 트레이 메뉴 "시작 시 자동 실행" 토글
- [ ] 등록 상태(현재 켜짐/꺼짐) 감지 및 표시
- [ ] README 의 자동 시작 섹션 갱신

## 향후 검토 항목 (백로그)

- 단축키 커스터마이즈: ESC 외 다른 키 또는 조합 지원
- 한글 외 IME(일본어/중국어) 호환성 확인 — 현재는 한글 IME 가정
- ESC를 빈번히 쓰는 게임/풀스크린 앱과의 상호작용 검증
- 오류 발생 시 트레이 풍선 알림 / 자동 복구
- 코드 사이닝 (글로벌 키보드 후킹은 안티바이러스 오탐 가능성)
- 자동 업데이트 메커니즘

## 배포 방식

당분간 로컬 `cargo build --release` 후 `target\release\esc-eng-ime.exe` 직접 사용. GitHub Releases 등 자동 배포는 미정.

## 변경 이력

- 2026-05-07: 초기 문서 3종(`README.md`, `AGENTS.md`, `PLAN.md`) 작성. 핵심 동작은 이미 구현 완료된 상태.
