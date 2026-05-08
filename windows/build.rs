// build.rs — Windows 리소스(트레이 아이콘 + VERSIONINFO) 임베드.
//
// CARGO_PKG_VERSION 을 읽어 VERSIONINFO 가 포함된 .rc 를 OUT_DIR 에 자동 생성한 뒤
// embed-resource crate 가 MSVC(rc.exe)/GNU(windres) 양쪽 툴체인에서 .rc → .res/.coff
// 컴파일을 자동 처리한다. 결과적으로 Cargo.toml 의 version 만 바꾸면 exe 의 파일
// 속성(Windows 파일 속성 창) 버전도 함께 갱신된다.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION 미설정");
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.first().copied().unwrap_or("0");
    let minor = parts.get(1).copied().unwrap_or("0");
    let patch = parts.get(2).copied().unwrap_or("0");
    let comma_version = format!("{},{},{},0", major, minor, patch);
    let dotted_version = format!("{}.{}.{}.0", major, minor, patch);

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 미설정");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR 미설정");

    // OUT_DIR 에 icon.ico 를 복사해 .rc 가 같은 디렉토리에서 참조하도록 함.
    // (rc.exe / windres 의 include 경로 의존을 피하기 위함)
    let src_ico = Path::new(&manifest_dir).join("assets").join("icon.ico");
    let dst_ico = Path::new(&out_dir).join("icon.ico");
    fs::copy(&src_ico, &dst_ico).expect("icon.ico 복사 실패");

    // VERSIONINFO + ICON 통합 .rc 작성. 언어/코드페이지 040904B0 = US English / Unicode.
    let rc_content = format!(
        r#"#include <winver.h>

1 ICON "icon.ico"

1 VERSIONINFO
FILEVERSION    {comma}
PRODUCTVERSION {comma}
FILEFLAGSMASK  VS_FFI_FILEFLAGSMASK
FILEFLAGS      0x0L
FILEOS         VOS_NT_WINDOWS32
FILETYPE       VFT_APP
FILESUBTYPE    VFT2_UNKNOWN
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName",      "kil9"
            VALUE "FileDescription",  "ESC key forces Korean IME to English mode"
            VALUE "FileVersion",      "{dotted}"
            VALUE "InternalName",     "eng-on-esc"
            VALUE "LegalCopyright",   "Copyright (C) 2026 kil9"
            VALUE "OriginalFilename", "eng-on-esc.exe"
            VALUE "ProductName",      "eng-on-esc"
            VALUE "ProductVersion",   "{dotted}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 0x04B0
    END
END
"#,
        comma = comma_version,
        dotted = dotted_version,
    );

    let rc_path = Path::new(&out_dir).join("app.rc");
    fs::write(&rc_path, rc_content).expect("app.rc 작성 실패");

    embed_resource::compile(&rc_path, embed_resource::NONE);

    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
}
