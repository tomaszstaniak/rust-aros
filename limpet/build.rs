use std::{env, path::Path, process::Command};
fn main() {
    println!("cargo:rustc-check-cfg=cfg(target_os, values(\"aros\"))");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=csrc");
    for variable in ["AROS_GCC", "AROS_SDK", "AROS_AR", "HOST_CC", "HOST_AR"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    let aros = env::var("CARGO_CFG_TARGET_OS").unwrap() == "aros";
    let cc = env::var(if aros { "AROS_GCC" } else { "HOST_CC" }).unwrap_or_else(|_| {
        if aros {
            panic!("set AROS_GCC to the selected AROS compiler")
        } else {
            "cc".into()
        }
    });
    let ar = env::var(if aros { "AROS_AR" } else { "HOST_AR" }).unwrap_or_else(|_| {
        if aros {
            format!(
                "{}ar",
                cc.strip_suffix("gcc")
                    .expect("set AROS_AR for a non-gcc compiler")
            )
        } else {
            "ar".into()
        }
    });
    let sdk = if aros {
        Some(env::var("AROS_SDK").expect("set AROS_SDK to the matching SDK"))
    } else {
        None
    };
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&root);
    let out = env::var("OUT_DIR").unwrap();
    let mut files = vec!["vendor/sdk/src/limpet_format.c"];
    if aros {
        files.extend(["vendor/sdk/src/limpet_query.c", "csrc/output.c"]);
    }
    let mut objects = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let object = format!("{out}/limpet-{index}.o");
        let mut command = Command::new(&cc);
        if let Some(sdk) = &sdk {
            command.arg(format!("--sysroot={sdk}"));
        } else {
            command.arg(format!(
                "-I{}",
                root.join("vendor/sdk/tests/host").display()
            ));
        }
        let result = command
            .args(["-O2", "-std=gnu99", "-Wall", "-Wextra", "-Werror"])
            .arg(format!("-I{}", root.join("vendor/sdk/include").display()))
            .arg("-c")
            .arg(root.join(file))
            .args(["-o", &object])
            .status()
            .expect("run C compiler");
        assert!(result.success(), "failed to compile {file}");
        objects.push(object);
    }
    let archive = Path::new(&out).join("libaroslimpet.a");
    if archive.exists() {
        std::fs::remove_file(&archive).unwrap();
    }
    assert!(Command::new(ar)
        .arg("rcs")
        .arg(&archive)
        .args(objects)
        .status()
        .expect("run archiver")
        .success());
    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=aroslimpet");
}
