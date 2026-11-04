use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=csrc/posix_probe.c");
    for name in ["AROS_GCC", "AROS_SDK", "AROS_AR"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("aros") {
        return;
    }
    let gcc = env::var("AROS_GCC").expect("AROS_GCC required");
    let sdk = env::var("AROS_SDK").expect("AROS_SDK required");
    let ar = env::var("AROS_AR").expect("AROS_AR required");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let object = out.join("posix_probe.o");
    assert!(Command::new(gcc)
        .args([format!("--sysroot={sdk}"), format!("-I{sdk}/include")])
        .args(["-O2", "-std=gnu99", "-c", "csrc/posix_probe.c", "-o"])
        .arg(&object)
        .status()
        .unwrap()
        .success());
    let archive = out.join("libposix_probe.a");
    if archive.exists() {
        std::fs::remove_file(&archive).unwrap();
    }
    assert!(Command::new(ar)
        .arg("rcs")
        .arg(archive)
        .arg(object)
        .status()
        .unwrap()
        .success());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=posix_probe");
}
