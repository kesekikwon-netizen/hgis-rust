//! 정적 PROJ(proj-sys bundled_proj)는 Windows 에서 사용자 폴더를 찾으려고 SHGetKnownFolderPath(shell32)·
//! CoTaskMemFree(ole32)를 부르는데 proj-sys 가 이 둘을 연결하지 않는다.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=ole32");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
