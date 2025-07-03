use core::panic;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=bootloader/aarch64.lds");
    println!("cargo:rerun-if-env-changed=XTASK_BUILD");

    println!("cargo:rustc-link-arg=-Tbootloader/aarch64.lds");

    // xtaskから呼ばれているかのチェック
    if !std::env::var("XTASK_BUILD").is_ok() {
        panic!(
            "
            Do not use `cargo build` directly.\n\
            Instead, run `cargo xtask build` or `cargo xbuild` from the workspace root.\n\
            "
        )
    }
}
