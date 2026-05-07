# Changelog

이 프로젝트의 모든 의미 있는 변경사항을 이 파일에 기록한다.

형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를 따르며,
버전 번호는 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)을 따른다.

## [Unreleased]

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

[Unreleased]: https://github.com/kil9/eng-on-esc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kil9/eng-on-esc/releases/tag/v0.1.0
