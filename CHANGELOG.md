# Changelog

이 프로젝트의 모든 의미 있는 변경사항을 이 파일에 기록한다.

형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를 따르며,
버전 번호는 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)을 따른다.

## [Unreleased]

## [0.4.0] - 2026-05-08

### Added
- **macOS 지원** (M3). `rdev` 기반 글로벌 키보드 후킹(CGEventTap), Caps Lock 자체 상태 머신으로 한국어 IME 추적. ESC 입력 시 한국어 모드이면 Caps Lock 시뮬레이션으로 영문 전환.
- macOS 메뉴바 아이콘 상주 (`tray-icon 0.19` + `tao 0.30`). 종료 메뉴, Dock 아이콘 없는 메뉴바 전용 앱(`NSApplicationActivationPolicyAccessory`).
- macOS 시작 시 자동 실행 토글. `~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist` 생성/삭제, 메뉴 항목 체크 표시로 상태 노출.
- GitHub Releases 에 `eng-on-esc-macos` 바이너리 포함 (`SHA256SUMS-macos.txt` 함께 업로드).

### Changed
- `README.md` 에 macOS 설치(Gatekeeper/손쉬운 사용 권한 안내) 및 사용법 섹션 추가.

## [0.3.0] - 2026-05-08

### Added
- 트레이 메뉴에 **시작 시 자동 실행** 토글 추가 (M2). 클릭하면 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 의 `eng-on-esc` 값이 추가/삭제되며, 메뉴 항목 앞에 체크 표시(`MF_CHECKED`)로 현재 상태가 보인다. 관리자 권한 불필요.

### Changed
- `README.md` 를 사용자 관점으로 재구성. 빌드/cargo 명령·`embed-resource`·`HKCU\Run` 같은 내부 디테일은 걷어내고 다운로드 → SmartScreen 안내 → 사용법 흐름으로 단순화. SmartScreen 안내에 "추가 정보 → 실행" 클릭 순서와 "게시자: 알 수 없음" 표시가 정상이라는 설명 추가.

## [0.2.0] - 2026-05-08

### Changed
- 프로젝트 이름을 `esc-eng-ime` 에서 `eng-on-esc` 로 변경. 산출물 파일명도 `eng-on-esc.exe` 로 바뀐다.
- Cargo 패키지명, 윈도우 클래스명, VERSIONINFO 메타데이터(`InternalName`/`OriginalFilename`/`ProductName`), GitHub Actions 릴리즈 워크플로 산출물명, 문서 일괄 갱신.

## [0.1.0] - 2026-05-08

첫 공개 릴리즈. ESC 키로 한글 IME 를 영문 모드로 전환하는 핵심 동작이 안정화됨.

### Added
- ESC 키다운 감지 후 IME 를 영문 모드로 강제 전환 (글로벌 LL 키보드 후킹 기반).
- ESC 본래 동작은 가로채지 않고 그대로 통과.
- thread별 IME 상태를 자체 상태머신으로 추적해 창 전환 시에도 일관 동작.
- 시스템 트레이 상주 + 좌/우 클릭으로 종료 메뉴 표시.
- ESC 키 캡 모양 전용 트레이 아이콘 (16/24/32/48/64/256 멀티 사이즈, exe 에 임베드).
- Windows 파일 속성 메타데이터(VERSIONINFO) — Cargo.toml 의 version 과 자동 동기화.
- GitHub Actions 릴리즈 워크플로 (`v*` 태그 push 트리거, unsigned exe + SHA256 체크섬 업로드).

### Known Limitations
- 시작 시 한국어 모드인 thread 의 첫 ESC 는 무동작 (한 번 한/영키 토글로 동기화).
- 마우스 작업표시줄 IME 토글, `Win+Space` 입력 언어 변경 등 키보드 외 경로는 추적 불가.
- 코드 사이닝이 적용되지 않은 unsigned 바이너리이므로 처음 실행 시 SmartScreen 경고가 표시될 수 있다.

[Unreleased]: https://github.com/kil9/eng-on-esc/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/kil9/eng-on-esc/releases/tag/v0.4.0
[0.3.0]: https://github.com/kil9/eng-on-esc/releases/tag/v0.3.0
[0.2.0]: https://github.com/kil9/eng-on-esc/releases/tag/v0.2.0
[0.1.0]: https://github.com/kil9/eng-on-esc/releases/tag/v0.1.0
