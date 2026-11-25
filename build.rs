use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_windows_kits_root() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files (x86)\Windows Kits\10",
        r"C:\Program Files\Windows Kits\10",
    ];
    for c in candidates {
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

fn compile_rc_with_sdk(rc_file: &Path, out_res: &Path) -> Result<(), String> {
    let kits = find_windows_kits_root().ok_or("Windows Kits 10 directory not found")?;
    let bin_base = kits.join("bin");
    let inc_base = kits.join("Include");

    let bin_ver = latest_version_dir(&bin_base, r"x64\rc.exe")
        .ok_or("Could not find rc.exe in Windows Kits 10\\bin")?;
    let inc_ver = latest_version_dir(&inc_base, r"um\windows.h")
        .ok_or("Could not find windows.h in Windows Kits 10\\Include")?;

    let rc_exe = bin_base.join(&bin_ver).join("x64").join("rc.exe");
    let inc_um = inc_base.join(&inc_ver).join("um");
    let inc_shared = inc_base.join(&inc_ver).join("shared");
    let inc_ucrt = inc_base.join(&inc_ver).join("ucrt");
    let src_dir = rc_file.parent().unwrap_or_else(|| Path::new("src"));

    let status = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(out_res)
        .arg("/I")
        .arg(src_dir)
        .arg("/I")
        .arg(&inc_um)
        .arg("/I")
        .arg(&inc_shared)
        .arg("/I")
        .arg(&inc_ucrt)
        .arg(rc_file)
        .status()
        .map_err(|e| format!("Failed to invoke rc.exe: {e}"))?;

    if !status.success() {
        return Err(format!("rc.exe exited with status {status}"));
    }
    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=src/resource.rc");
    println!("cargo:rerun-if-changed=src/resource.h");
    println!("cargo:rerun-if-changed=src/res/100.ico");
    println!("cargo:rerun-if-changed=src/res/manifest.xml");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc_file = manifest_dir.join("src").join("resource.rc");
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_env == "msvc" {
        let out_res = out_dir.join("resource.res");
        compile_rc_with_sdk(&rc_file, &out_res).expect("Failed to compile Win32 resources");
        println!("cargo:rustc-link-arg-bins={}", out_res.display());
    } else {
        let out_obj = out_dir.join("resource.o");
        let status = Command::new("windres")
            .arg("-I")
            .arg(manifest_dir.join("src"))
            .arg("-i")
            .arg(&rc_file)
            .arg("-o")
            .arg(&out_obj)
            .status()
            .expect("Failed to invoke windres");
        assert!(status.success(), "windres failed");
        println!("cargo:rustc-link-arg-bins={}", out_obj.display());
    }
}
