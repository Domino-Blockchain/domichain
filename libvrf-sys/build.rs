fn main() {
    pkg_config::Config::new()
        .atleast_version("1.0.18")
        .probe("libsodium")
        .unwrap();
    let src = [
        "src/vrf.c",
    ];
    let mut builder = cc::Build::new();
    let build = builder
        .files(src.iter())
        .include("include")
        .flag("-Wno-unused-parameter");
    build.compile("vrf");
}
