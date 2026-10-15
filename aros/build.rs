// Compile csrc/execglue.c with the AROS cross compiler and link it in, so
// exec.library is reached through proto/exec.h rather than through libexec.a's
// stub symbols. See csrc/execglue.c for why.
//
// The toolchain is named explicitly by AROS_GCC and AROS_SDK, which setup.sh
// writes into aros-env.toml. That is a hard requirement, not a convenience:
// this file is compiled with a particular SDK's headers, and the inline
// functions in proto/exec.h differ *between AROS builds*. AROS One 1.3 passes
// SysBase in %r12 through a register-call ABI; mainline passes it as an
// ordinary third argument in %rdx. Compile the glue against one and link it
// against the other and the program dies inside AllocMem, which reads exactly
// like a broken startup and is not one.
//
// An earlier version tried to read the paths out of the target spec, on the
// assumption that TARGET holds the path to the .json. Cargo passes the target
// *name*, so that branch never fired and the environment silently decided the
// toolchain — which is how the two SDKs got mixed in the first place.
use std::{env, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=csrc/execglue.c");
    println!("cargo:rerun-if-env-changed=AROS_GCC");
    println!("cargo:rerun-if-env-changed=AROS_SDK");
    println!("cargo:rerun-if-env-changed=AROS_AR");

    let gcc = require("AROS_GCC", "the AROS cross compiler, e.g. .../x86_64-aros-gcc");
    let sysroot = require("AROS_SDK", "the AROS SDK directory");

    let out = env::var("OUT_DIR").unwrap();
    let src = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("csrc/execglue.c");
    let obj = format!("{out}/execglue.o");

    let status = Command::new(&gcc)
        .args([&format!("--sysroot={sysroot}"), &format!("-I{sysroot}/include"), "-O2", "-c"])
        .arg(&src)
        .args(["-o", &obj])
        .status()
        .unwrap_or_else(|e| panic!("could not run {gcc}: {e}"));
    assert!(status.success(), "execglue.c failed to compile with {gcc}");

    let ar = env::var("AROS_AR").unwrap_or_else(|_| {
        let prefix = gcc.strip_suffix("gcc")
            .expect("set AROS_AR explicitly when AROS_GCC is not a gcc driver");
        format!("{prefix}ar")
    });
    let status = Command::new(&ar)
        .args(["rcs", &format!("{out}/libarosexecglue.a"), &obj])
        .status()
        .unwrap_or_else(|e| panic!("could not run {ar}: {e}"));
    assert!(status.success(), "could not archive execglue.o");

    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=arosexecglue");
}

fn require(var: &str, what: &str) -> String {
    env::var(var).unwrap_or_else(|_| {
        panic!(
            "{var} is not set. It must name {what}. \
             Run ./setup.sh to write aros-env.toml, or set it yourself — and make sure \
             it names the same AROS build the linker in your target spec belongs to."
        )
    })
}
