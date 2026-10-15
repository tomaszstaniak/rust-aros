// Compile csrc/compat.c with the explicitly selected AROS cross compiler
// and SDK, and link it in. This gives std the sockets, *at() calls and stubs
// that the SDK's C library does not have as plain symbols.
use std::{env, fs, path::Path, process::Command};

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    // Cargo TARGET is a target name, not the path to its JSON specification.
    // C glue must use the same explicit inputs as the target linker.
    for name in ["AROS_GCC", "AROS_SDK", "AROS_AR"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let gcc = env::var("AROS_GCC").expect("set AROS_GCC to the selected AROS cross compiler");
    let sysroot = env::var("AROS_SDK").expect("set AROS_SDK to its matching SDK directory");
    let ar = env::var("AROS_AR").unwrap_or_else(|_| {
        let prefix = gcc.strip_suffix("gcc")
            .expect("set AROS_AR explicitly when AROS_GCC is not a gcc driver");
        format!("{prefix}ar")
    });
    let csrc = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("csrc");
    let mut sources = vec![csrc.join("compat.c"), csrc.join("exec_adapter.c"), csrc.join("entropy.c"), csrc.join("jent_adapter.c")];
    let vendor = csrc.join("vendor/jitterentropy");
    let mut core: Vec<_> = fs::read_dir(vendor.join("src")).expect("vendored jitterentropy sources missing")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "c"))
        .collect();
    core.sort();
    sources.extend(core);
    let mut objects = Vec::new();
    for (index, src) in sources.iter().enumerate() {
        let obj = format!("{out}/aroscompat-{index}.o");
        // jitterentropy deliberately refuses optimization; preserve arospkg's
        // upstream-required -O0/-fwrapv recipe for the core sources only.
        let optimization = if src.starts_with(&vendor) { "-O0" } else { "-O2" };
        let status = Command::new(&gcc)
            .args([format!("--sysroot={sysroot}"), format!("-I{sysroot}/include"),
                   format!("-I{}", vendor.display()), format!("-I{}", vendor.join("src").display()),
                   format!("-I{}", csrc.join("vendor/arospkg-entropy").display())])
            .args([optimization, "-std=gnu99", "-fwrapv", "-c"])
            .arg(src).args(["-o", &obj]).status().expect("aros compiler not runnable");
        assert!(status.success(), "failed to compile {}", src.display());
        objects.push(obj);
    }
    // ar rcs retains unnamed members. Remove the old archive so deleted or
    // renamed compatibility sources cannot survive a Cargo rebuild.
    let archive = Path::new(&out).join("libaroscompat.a");
    if archive.exists() { fs::remove_file(&archive).expect("remove stale compatibility archive"); }
    assert!(Command::new(&ar).arg("rcs").arg(format!("{out}/libaroscompat.a"))
        .args(&objects).status().expect("aros archiver not runnable").success());
    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=aroscompat");
    // crt before net: net.lib has its own strerror, which asks
    // bsdsocket.library through SocketBase and crashes when that is NULL
    // (no TCP/IP stack). Listed first, crt's strerror satisfies the symbol.
    println!("cargo:rustc-link-lib=crt");
    println!("cargo:rustc-link-lib=net");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rerun-if-changed=csrc");
}
