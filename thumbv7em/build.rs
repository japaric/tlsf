fn main() {
    println!("cargo:rustc-link-arg-examples=-Tlink.x");
    println!("cargo:rustc-link-arg-examples=-Tdefmt.x");
}
