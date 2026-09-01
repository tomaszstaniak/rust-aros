// Compile csrc/compat.c with the AROS cross compiler named in the target
// spec and link it in. This gives std the sockets, *at() calls and stubs
// that the SDK's C library does not have as plain symbols.
use std::{env, fs, path::Path, process::Command};

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    // The toolchain comes from the target spec when cargo hands us its path
    // (custom targets do), or from AROS_GCC / AROS_SDK in the environment.
    let (gcc, sysroot) = match env::var("TARGET").ok().filter(|t| t.ends_with(".json")).map(|t| fs::read_to_string(t).ok()).flatten() {
        Some(text) => (
            text.lines().find(|l| l.contains("\"linker\"")).unwrap().split('"').nth(3).unwrap().to_string(),
            text.lines().find(|l| l.contains("--sysroot=")).unwrap()
                .split("--sysroot=").nth(1).unwrap().trim_end_matches(['"', ',', ' ']).to_string(),
        ),
        None => (
            env::var("AROS_GCC").expect("set AROS_GCC to x86_64-aros-gcc"),
            env::var("AROS_SDK").expect("set AROS_SDK to the SDK directory"),
        ),
    };
    let spec = Path::new("csrc/compat.c").to_path_buf();
    let src = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("csrc/compat.c");
    let obj = format!("{out}/compat.o");
    let st = Command::new(&gcc)
        .args([&format!("--sysroot={sysroot}"), &format!("-I{sysroot}/include"), "-O2", "-c"])
        .arg(&src).args(["-o", &obj]).status().expect("aros gcc not runnable");
    assert!(st.success(), "compat.c failed to compile");
    let ar = gcc.replace("gcc", "ar");
    assert!(Command::new(&ar).args(["rcs", &format!("{out}/libaroscompat.a"), &obj]).status().unwrap().success());
    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=aroscompat");
    println!("cargo:rustc-link-lib=net");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rerun-if-changed=csrc/compat.c");
}
