# eng-on-esc

ESC 키를 누르면 IME를 영문 모드로 강제 전환해 주는 작은 상주 유틸리티. Windows와 macOS를 지원한다.

## 이게 뭔가요?

한글 IME가 켜진 상태에서 ESC를 눌러도 IME 모드는 그대로 남아 있어 다음 입력이 다시 한글로 시작되는 일이 잦다. 이 프로그램은 ESC 키 입력을 감지해 IME만 영문 모드로 바꿔 주고, ESC 본래 동작(취소/닫기 등)은 그대로 통과시킨다. 트레이(메뉴바)에 작은 아이콘으로 상주한다.

## 다운로드

[GitHub Releases](https://github.com/kil9/eng-on-esc/releases) 페이지에서 최신 버전을 받는다.

- **Windows**: `eng-on-esc-windows.exe`
- **macOS**: `eng-on-esc-macos`

설치 과정 없이 단독 실행 파일이라 원하는 폴더에 두고 실행하면 된다.

## Windows — 처음 실행할 때 (SmartScreen 경고)

코드 사이닝 인증서가 적용되지 않은 바이너리라, 처음 실행하면 다음과 같은 화면을 만날 수 있다.

- **"Windows의 PC 보호"** 파란 창이 뜨면 → 작은 글씨 **추가 정보** 클릭 → 하단에 나타나는 **실행** 버튼 클릭
- 게시자가 **"알 수 없음"** 으로 표시되는 것은 정상이다 (코드 사이닝을 하지 않아서)

처음 한 번만 허용하면 이후로는 경고 없이 실행된다.

(선택) 다운로드한 파일이 변조되지 않았는지 확인하려면 같은 Release 페이지의 `SHA256SUMS-windows.txt` 와 비교한다. PowerShell:

```powershell
Get-FileHash eng-on-esc-windows.exe -Algorithm SHA256
```

## macOS — 처음 실행할 때

### Gatekeeper 허용

코드 사이닝이 없어 처음 실행하면 **"개발자를 확인할 수 없기 때문에 열 수 없습니다"** 경고가 뜬다.

1. Finder에서 파일을 **우클릭(컨트롤+클릭) → 열기** 선택
2. 경고 창에서 **열기** 클릭
3. 이후로는 경고 없이 실행된다

또는 터미널에서:

```sh
xattr -d com.apple.quarantine eng-on-esc-macos
chmod +x eng-on-esc-macos
./eng-on-esc-macos
```

### 손쉬운 사용(Accessibility) 권한

글로벌 키보드 후킹에 손쉬운 사용 권한이 필요하다. 처음 실행 시 권한 요청 창이 뜨거나, 메뉴바 아이콘이 나타나지 않는 경우:

**시스템 설정 → 개인 정보 보호 및 보안 → 손쉬운 사용** 에서 `eng-on-esc-macos` 를 허용한다.

(선택) SHA256 검증:

```sh
shasum -a 256 eng-on-esc-macos
```

`SHA256SUMS-macos.txt` 의 값과 일치해야 한다.

## 사용법

실행하면 **Windows는 시스템 트레이**, **macOS는 메뉴바** 에 작은 아이콘이 나타난다. 별도 창은 뜨지 않는다.

이제 ESC 를 누르기만 하면 된다. ESC 본래 동작(취소/닫기 등)은 그대로 동작하면서, IME 가 한국어 모드였다면 자동으로 영문 모드로 바뀐다.

### 트레이/메뉴바 메뉴

아이콘을 클릭하면 메뉴가 뜬다.

- **시작 시 자동 실행** — 한 번 누르면 부팅 시 자동 실행이 켜지고(✓ 표시), 다시 누르면 꺼진다. 관리자 권한 없이 동작한다.
  - Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 에 등록
  - macOS: `~/Library/LaunchAgents/io.github.kil9.eng-on-esc.plist` 생성
- **종료** — 프로그램을 끝낸다.

> 바이너리를 다른 폴더로 옮긴 뒤에는 메뉴를 한 번 OFF → ON 다시 토글해 새 경로로 갱신해야 한다.

## 알아 두면 좋은 한계

다음 경우엔 ESC 를 눌러도 영문 전환이 한 번 누락될 수 있다. 이때는 한/영키(Windows) 또는 Caps Lock(macOS)을 한 번 눌러 동기화하면 이후로는 정상 동작한다.

- 프로그램 실행 직후, 이미 한국어 모드인 상태에서의 첫 ESC
- 키보드 외 경로(마우스 작업표시줄 클릭, `Win+Space`, 입력 소스 직접 변경 등)로 IME를 바꾼 직후

## 라이선스 / 소스

오픈소스다. 소스 코드와 빌드 방법은 [GitHub 저장소](https://github.com/kil9/eng-on-esc) 참조.
