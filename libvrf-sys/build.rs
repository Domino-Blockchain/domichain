fn main() {
    pkg_config::Config::new()
        .atleast_version("1.0.18")
        .probe("libsodium")
        .unwrap();
    let src = ["src/vrf.c"];
    let mut builder = cc::Build::new();
    let build = builder
        .files(src.iter())
        .include("include")
        .include("/opt/homebrew/Cellar/libsodium/1.0.18_1/include") //Add this line to compile successfully on MacOS
        .flag("-Wno-unused-parameter");
    build.compile("vrf");

    println!("cargo:rerun-if-changed=src/vrf.c");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/bindings.rs");
    println!("cargo:rerun-if-changed=include/vrf.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
