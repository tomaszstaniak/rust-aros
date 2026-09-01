// Compile the C glue with the AROS cross compiler taken from the target spec's
// linker entry, so the same toolchain builds both halves.
use std::{env, fs, process::Command};

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    let spec = fs::read_to_string("../../x86_64-aros.json").expect("run setup.sh first");
    let gcc = spec.lines().find(|l| l.contains("\"linker\"")).unwrap()
        .split('"').nth(3).unwrap().to_string();
    let sysroot = spec.lines().find(|l| l.contains("--sysroot=")).unwrap()
        .split("--sysroot=").nth(1).unwrap().trim_end_matches(['"', ',', ' ']).to_string();
    let obj = format!("{out}/sockglue.o");
    let st = Command::new(&gcc)
        .args([&format!("--sysroot={sysroot}"), &format!("-I{sysroot}/include"), "-O2", "-c",
               "csrc/sockglue.c", "-o", &obj])
        .status().expect("aros gcc");
    assert!(st.success(), "sockglue.c failed to compile");
    let ar = gcc.replace("gcc", "ar");
    assert!(Command::new(&ar).args(["rcs", &format!("{out}/libsockglue.a"), &obj]).status().unwrap().success());
    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=sockglue");
    println!("cargo:rustc-link-lib=net");          // libnet.a: opens bsdsocket.library at startup
    println!("cargo:rerun-if-changed=csrc/sockglue.c");
}
