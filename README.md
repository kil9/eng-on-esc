# esc-eng-ime

Windows에서 ESC 키를 누르면 IME를 영문 모드로 강제 전환해 주는 작은 상주 유틸리티.

## 개요

한글 IME가 켜진 상태에서 ESC를 눌러도 IME 모드는 그대로 남아 있어 다음 입력이 다시 한글로 시작되는 경우가 잦다. 이 프로그램은 ESC 키 입력을 감지해 IME만 영문 모드로 바꿔 주고, ESC 본래 동작(취소/닫기 등)은 그대로 통과시킨다. 시스템 트레이에 상주하며 트레이 아이콘 클릭으로 종료할 수 있다.

## 사전 요구사항

- Windows (10/11)
- Rust toolchain (edition 2021)

## 빌드

```sh
cargo build --release
```

빌드 산출물: `target\release\esc-eng-ime.exe`

## 실행

빌드한 `.exe` 를 직접 실행하면 트레이 아이콘으로 상주한다.

```sh
cargo run --release
```

또는

```
target\release\esc-eng-ime.exe
```

### 자동 시작 (선택)

시작 프로그램 폴더에 `.exe` 의 바로가기를 두면 부팅 시 자동 실행된다.

```
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
```

`Win+R` → `shell:startup` 으로 해당 폴더를 바로 열 수 있다.

## 기능

- ESC 키다운 → 포그라운드 창의 IME 를 영문 모드로 강제 전환
- ESC 본래 동작은 그대로 통과
- 시스템 트레이 아이콘으로 상주
- 트레이 아이콘 좌/우 클릭 → 종료 메뉴
- 디버그 로그: `%TEMP%\esc-eng-ime.log`

## 종료

트레이 아이콘 클릭 → "종료" 선택.
