use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_windows_kits_root() -> Option<PathBuf> {
    for c in [
        r"C:\Program Files (x86)\Windows Kits\10",
        r"C:\Program Files\Windows Kits\10",
    ] {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn latest_version_dir(base: &Path, must_contain: &str) -> Option<String> {
    let mut versions: Vec<String> = fs::read_dir(base)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("10.") && e.path().join(must_contain).exists() {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    versions.sort();
    versions.pop()
}

fn main() {
    println!("cargo:rerun-if-changed=resource.rc");
    println!("cargo:rerun-if-changed=resource.h");
    println!("cargo:rerun-if-changed=manifest.xml");
    println!("cargo:rerun-if-changed=../target/release/ramreduct.exe");
    println!("cargo:rerun-if-changed=../bin/memreduct.lng");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc_file = manifest_dir.join("resource.rc");
    let out_res = out_dir.join("installer_resource.res");

    let kits = find_windows_kits_root().expect("Windows Kits 10 directory not found");
    let bin_base = kits.join("bin");
    let inc_base = kits.join("Include");

    let bin_ver = latest_version_dir(&bin_base, r"x64\rc.exe").expect("rc.exe not found");
    let inc_ver = latest_version_dir(&inc_base, r"um\windows.h").expect("windows.h not found");

    let rc_exe = bin_base.join(&bin_ver).join("x64").join("rc.exe");
    let status = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&out_res)
        .arg("/I")
        .arg(&manifest_dir)
        .arg("/I")
        .arg(inc_base.join(&inc_ver).join("um"))
        .arg("/I")
        .arg(inc_base.join(&inc_ver).join("shared"))
        .arg("/I")
        .arg(inc_base.join(&inc_ver).join("ucrt"))
        .arg(&rc_file)
        .status()
        .expect("Failed to run rc.exe");

    assert!(status.success(), "rc.exe failed");
    println!("cargo:rustc-link-arg-bins={}", out_res.display());
}
