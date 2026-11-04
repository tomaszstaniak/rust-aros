fn main() {
    let directory =
        std::env::var("AROS_TLS_LIB_DIR").expect("build with ports/async-network/tls-build.py");
    println!("cargo:rerun-if-env-changed=AROS_TLS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=AROS_TLS_FORCE_FAIL");
    println!("cargo:rustc-link-search=native={directory}");
    for lib in [
        "rust_aros_tls",
        "mbedtls",
        "mbedx509",
        "mbedcrypto",
        "mbedtls_aros",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    let adapter = if std::env::var_os("AROS_TLS_FORCE_FAIL").is_some() {
        "jitterentropy_mbedtls_fail"
    } else {
        "jitterentropy_mbedtls"
    };
    println!("cargo:rustc-link-lib=static={adapter}");
    println!("cargo:rustc-link-lib=static=jitterentropy");
}
