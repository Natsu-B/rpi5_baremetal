fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=bootloader/aarch64.lds");

    println!("cargo:rustc-link-arg=-Tbootloader/aarch64.lds");
}