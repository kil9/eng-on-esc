# esc-eng-ime

Windows에서 ESC 키를 누르면 IME를 영문 모드로 강제 전환해 주는 작은 상주 유틸리티.

## 개요

한글 IME가 켜진 상태에서 ESC를 눌러도 IME 모드는 그대로 남아 있어 다음 입력이 다시 한글로 시작되는 경우가 잦다. 이 프로그램은 ESC 키 입력을 감지해 IME만 영문 모드로 바꿔 주고, ESC 본래 동작(취소/닫기 등)은 그대로 통과시킨다. 시스템 트레이에 상주하며 트레이 아이콘 클릭으로 종료할 수 있다.

## 다운로드 / 설치

미리 빌드된 바이너리는 [GitHub Releases](https://github.com/kil9/eng-on-esc/releases) 에서 받을 수 있다. 별도 설치 과정 없이 `esc-eng-ime.exe` 단독 실행이면 충분하다.

> **SmartScreen 경고 안내** — 코드 사이닝이 적용되지 않은 unsigned 바이너리이므로 처음 실행하면 "Windows의 PC 보호" 경고가 뜰 수 있다. **추가 정보 → 실행** 을 차례로 누르면 진행된다.

(선택) 다운로드한 파일이 변조되지 않았는지 SHA256 으로 검증하려면 같은 Release 페이지의 `SHA256SUMS.txt` 와 비교한다. PowerShell:

```powershell
Get-FileHash esc-eng-ime.exe -Algorithm SHA256
```

출력 해시(소문자) 가 `SHA256SUMS.txt` 의 값과 일치해야 한다.

## 사전 요구사항 (직접 빌드 시)

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

### 트레이 아이콘 (icon.ico)

전용 아이콘은 `assets/icon.ico` 에 들어 있고 빌드 시 `embed-resource` 로 exe 안에 임베드된다. 별도로 `.ico` 파일을 같이 배포할 필요 없이 `esc-eng-ime.exe` 단독으로 실행 가능하다.

아이콘을 새로 그리고 싶다면 PowerShell 에서 아래를 실행해 `assets/icon.ico` 를 재생성한 뒤 `cargo build --release` 로 다시 빌드한다.

```powershell
pwsh -File tools/generate-icon.ps1
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
- 창(thread)별로 IME 상태를 독립 추적 — 창 전환 후에도 의도대로 동작
- 시스템 트레이 아이콘으로 상주
- 트레이 아이콘 좌/우 클릭 → 종료 메뉴

## 동작 방식과 한계

내부적으로 LL 키보드 후킹으로 사용자가 직접 누른 한/영키(`VK_HANGUL`)를 추적해 thread별 IME 모드를 자체적으로 보관한다. ESC 시 그 모드가 한국어이면 한/영키 시뮬레이션을 보내 영문으로 전환한다.

따라서 다음 경우 한 번 한/영키를 눌러 동기화해야 한다:

- 프로그램 실행 직후, 이미 한국어 모드인 창에서의 첫 ESC
- 마우스로 작업표시줄 IME 토글, `Win+Space` 입력 언어 변경 등 키보드 외 경로로 모드를 바꾼 직후

## 종료

트레이 아이콘 클릭 → "종료" 선택.
