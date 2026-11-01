fn main() {
    println!("cargo:rerun-if-changed=ui/");
    println!("cargo:rerun-if-changed=examples/crypto_tracker.slint");
    println!("cargo:rerun-if-changed=examples/pos_terminal.slint");
    println!("cargo:rerun-if-changed=examples/dev_forge.slint");

    slint_build::compile("ui/app.slint").expect("Failed to compile Slint UI files");

    // Compile standalone isolated examples without bloating main crate
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    if std::path::Path::new("examples/crypto_tracker.slint").exists() {
        let _ = slint_build::compile_with_output_path(
            "examples/crypto_tracker.slint",
            out_dir.join("crypto_tracker.rs"),
            slint_build::CompilerConfiguration::new(),
        );
    }
    if std::path::Path::new("examples/pos_terminal.slint").exists() {
        slint_build::compile_with_output_path(
            "examples/pos_terminal.slint",
            out_dir.join("pos_terminal.rs"),
            slint_build::CompilerConfiguration::new(),
        ).expect("Failed to compile examples/pos_terminal.slint");
    }
    if std::path::Path::new("examples/dev_forge.slint").exists() {
        slint_build::compile_with_output_path(
            "examples/dev_forge.slint",
            out_dir.join("dev_forge.rs"),
            slint_build::CompilerConfiguration::new(),
        ).expect("Failed to compile examples/dev_forge.slint");
    }

    #[cfg(windows)]
    embed_windows_icon(&out_dir);
}

#[cfg(windows)]
fn embed_windows_icon(out_dir: &std::path::Path) {
    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => return,
    };
    let ico_path = manifest_dir.join("assets").join("app-icon.ico");
    if !ico_path.exists() {
        return;
    }
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    let ico_str = ico_path.to_str().unwrap().replace('\\', "/");
    let rc_path = out_dir.join("app.rc");
    let res_path = out_dir.join("app.res");

    let rc_content = format!("1 ICON \"{}\"\n", ico_str);
    if std::fs::write(&rc_path, rc_content).is_err() {
        return;
    }

    // Locate rc.exe (Windows SDK Resource Compiler)
    let mut rc_cmd = None;
    if std::process::Command::new("rc.exe").arg("/?").output().is_ok() {
        rc_cmd = Some(std::path::PathBuf::from("rc.exe"));
    } else {
        let kits_dir = std::path::Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
        if kits_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(kits_dir) {
                let mut versions: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .collect();
                versions.sort_by_key(|e| e.file_name());
                versions.reverse();

                for ver in versions {
                    let candidate = ver.path().join("x64").join("rc.exe");
                    if candidate.exists() {
                        rc_cmd = Some(candidate);
                        break;
                    }
                }
            }
        }
    }

    if let Some(rc_exe) = rc_cmd {
        let status = std::process::Command::new(rc_exe)
            .arg("/fo")
            .arg(&res_path)
            .arg(&rc_path)
            .status();

        if let Ok(s) = status {
            if s.success() && res_path.exists() {
                // Pass compiled .res to MSVC linker for binaries and examples
                println!("cargo:rustc-link-arg-bins={}", res_path.display());
                println!("cargo:rustc-link-arg-examples={}", res_path.display());
            }
        }
    }
}

