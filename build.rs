fn main() {
    println!("cargo:rustc-env=OUT_DIR={}", std::env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-env=PROFILE={}", std::env::var("PROFILE").unwrap());
}
