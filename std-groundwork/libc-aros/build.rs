// Compile csrc/compat.c with the explicitly selected AROS cross compiler
// and SDK, and link it in. This gives std the sockets, *at() calls and stubs
// that the SDK's C library does not have as plain symbols.
use std::{env, path::Path, process::Command};

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    // Cargo TARGET is a target name, not the path to its JSON specification.
    // C glue must use the same explicit inputs as the target linker.
    for name in ["AROS_GCC", "AROS_SDK", "AROS_AR"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let gcc = env::var("AROS_GCC").expect("set AROS_GCC to the selected AROS cross compiler");
    let sysroot = env::var("AROS_SDK").expect("set AROS_SDK to its matching SDK directory");
    let src = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("csrc/compat.c");
    let obj = format!("{out}/compat.o");
    let st = Command::new(&gcc)
        .args([&format!("--sysroot={sysroot}"), &format!("-I{sysroot}/include"), "-O2", "-c"])
        .arg(&src).args(["-o", &obj]).status().expect("aros gcc not runnable");
    assert!(st.success(), "compat.c failed to compile");
    let ar = env::var("AROS_AR").unwrap_or_else(|_| {
        let prefix = gcc.strip_suffix("gcc")
            .expect("set AROS_AR explicitly when AROS_GCC is not a gcc driver");
        format!("{prefix}ar")
    });
    assert!(Command::new(&ar).args(["rcs", &format!("{out}/libaroscompat.a"), &obj]).status().unwrap().success());
    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=aroscompat");
    // crt before net: net.lib has its own strerror, which asks
    // bsdsocket.library through SocketBase and crashes when that is NULL
    // (no TCP/IP stack). Listed first, crt's strerror satisfies the symbol.
    println!("cargo:rustc-link-lib=crt");
    println!("cargo:rustc-link-lib=net");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rerun-if-changed=csrc/compat.c");
}
