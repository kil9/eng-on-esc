// build.rs — Windows 리소스(트레이 아이콘) 임베드.
//
// embed-resource crate 가 MSVC(rc.exe)/GNU(windres) 양쪽 툴체인에서
// .rc → .res/.coff 컴파일을 자동 처리한다.

fn main() {
    embed_resource::compile("assets/icon.rc", embed_resource::NONE);
}
