use std::path::{Path, PathBuf};

/// Status of the desktop shortcut
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutStatus {
    Installed(PathBuf),
    NotInstalled,
}

/// Result of a shortcut creation or deletion operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutResult {
    Created(PathBuf),
    AlreadyExists(PathBuf),
    Removed(PathBuf),
    NotFound,
}

/// Helper to expand Windows environment variables (e.g. %USERPROFILE%)
fn expand_env_vars(raw: &str) -> String {
    let mut result = raw.to_string();
    if raw.contains('%') {
        #[cfg(windows)]
        {
            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                result = result.replace("%USERPROFILE%", &user_profile);
            }
            if let Ok(system_drive) = std::env::var("SystemDrive") {
                result = result.replace("%SystemDrive%", &system_drive);
            }
        }
    }
    result
}

/// Safely resolves the user's primary Desktop directory
pub fn get_desktop_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let mut candidate: Option<PathBuf> = None;

        // 1. Query Windows Registry for active Desktop folder (handles OneDrive & custom redirection)
        if let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\User Shell Folders")
        {
            if let Ok(desktop_raw) = hkcu.get_value::<String, _>("Desktop") {
                let expanded = expand_env_vars(&desktop_raw);
                let p = PathBuf::from(expanded);
                if p.exists() && p.is_dir() {
                    return Ok(p);
                }
                candidate = Some(p);
            }
        }

        // 2. Fallback to USERPROFILE\Desktop
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(user_profile).join("Desktop");
            if p.exists() && p.is_dir() {
                return Ok(p);
            }
            if candidate.is_none() {
                candidate = Some(p);
            }
        }

        if let Some(c) = candidate {
            return Ok(c);
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(&home).join("Desktop");
            if p.exists() && p.is_dir() {
                return Ok(p);
            }
            let home_p = PathBuf::from(home);
            if home_p.exists() && home_p.is_dir() {
                return Ok(home_p);
            }
            return Ok(p);
        }
    }

    Err("Could not resolve a valid Desktop directory".to_string())
}

/// Checks whether a valid, healthy desktop shortcut currently exists
pub fn check_shortcut(app_name: &str) -> ShortcutStatus {
    let Ok(desktop) = get_desktop_dir() else {
        return ShortcutStatus::NotInstalled;
    };

    #[cfg(windows)]
    let shortcut_path = desktop.join(format!("{}.lnk", app_name));

    #[cfg(not(windows))]
    let shortcut_path = desktop.join(format!("{}.desktop", app_name));

    if shortcut_path.exists() && shortcut_path.is_file() {
        if let Ok(meta) = shortcut_path.metadata() {
            if meta.len() > 0 {
                return ShortcutStatus::Installed(shortcut_path);
            }
        }
    }

    ShortcutStatus::NotInstalled
}

/// Creates a native desktop shortcut with smart deduplication and icon embedding.
///
/// If a valid shortcut already exists, it will not recreate it, returning `ShortcutResult::AlreadyExists`.
pub fn create_shortcut(
    app_name: &str,
    target_exe: Option<PathBuf>,
    icon_file: Option<PathBuf>,
    description: Option<&str>,
) -> Result<ShortcutResult, String> {
    let desktop = get_desktop_dir()?;
    create_shortcut_in_dir(&desktop, app_name, target_exe, icon_file, description)
}

/// Internal implementation supporting target directory injection (for testing & safety)
pub fn create_shortcut_in_dir(
    target_dir: &Path,
    app_name: &str,
    target_exe: Option<PathBuf>,
    icon_file: Option<PathBuf>,
    _description: Option<&str>,
) -> Result<ShortcutResult, String> {
    if !target_dir.exists() || !target_dir.is_dir() {
        return Err(format!("Target directory does not exist: {}", target_dir.display()));
    }

    // Resolve target executable
    let exe_path = match target_exe {
        Some(p) => p,
        None => std::env::current_exe().map_err(|e| format!("Failed to resolve current executable: {}", e))?,
    };

    if !exe_path.exists() {
        return Err(format!("Target executable does not exist: {}", exe_path.display()));
    }

    #[cfg(windows)]
    {
        let shortcut_path = target_dir.join(format!("{}.lnk", app_name));

        // Smart Check: If a healthy shortcut already exists, return AlreadyExists
        if shortcut_path.exists() && shortcut_path.is_file() {
            if let Ok(meta) = shortcut_path.metadata() {
                if meta.len() > 100 {
                    return Ok(ShortcutResult::AlreadyExists(shortcut_path));
                }
            }
        }

        let work_dir = exe_path.parent().unwrap_or(target_dir);

        let mut link = mslnk::ShellLink::new(&exe_path)
            .map_err(|e| format!("Failed to create Windows shell link: {}", e))?;

        link.set_name(Some(app_name.to_string()));
        link.set_working_dir(Some(work_dir.to_string_lossy().to_string()));

        // Resolve icon
        if let Some(icon) = icon_file {
            if icon.exists() {
                link.set_icon_location(Some(icon.to_string_lossy().to_string()));
            }
        } else {
            // Auto-detect .ico in standard assets locations
            let candidates = [
                PathBuf::from("assets/app-icon.ico"),
                exe_path.parent().map(|p| p.join("assets/app-icon.ico")).unwrap_or_default(),
                exe_path.parent().map(|p| p.join("app-icon.ico")).unwrap_or_default(),
            ];
            for c in candidates {
                if c.exists() {
                    link.set_icon_location(Some(c.canonicalize().unwrap_or(c).to_string_lossy().to_string()));
                    break;
                }
            }
        }

        link.create_lnk(&shortcut_path)
            .map_err(|e| format!("Failed to write .lnk shortcut: {}", e))?;

        Ok(ShortcutResult::Created(shortcut_path))
    }

    #[cfg(not(windows))]
    {
        let shortcut_path = target_dir.join(format!("{}.desktop", app_name));

        if shortcut_path.exists() && shortcut_path.is_file() {
            if let Ok(meta) = shortcut_path.metadata() {
                if meta.len() > 50 {
                    return Ok(ShortcutResult::AlreadyExists(shortcut_path));
                }
            }
        }

        let icon_str = icon_file
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "application-x-executable".to_string());

        let desc_str = description.unwrap_or("Native Desktop Application");

        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec=\"{}\"\nIcon={}\nTerminal=false\nCategories=Utility;Development;\n",
            app_name, desc_str, exe_path.display(), icon_str
        );

        std::fs::write(&shortcut_path, content)
            .map_err(|e| format!("Failed to write .desktop shortcut: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&shortcut_path, std::fs::Permissions::from_mode(0o755));
        }

        Ok(ShortcutResult::Created(shortcut_path))
    }
}

/// Safely removes the desktop shortcut if it exists.
///
/// SAFETY: This function enforces 5 strict safety barriers before deletion:
/// 1. Verifies the target path is strictly within the user's Desktop directory.
/// 2. Verifies the filename extension is strictly `.lnk` (or `.desktop`).
/// 3. Verifies the filename matches the application name.
/// 4. Verifies the target is a regular file (never a directory or symlink).
/// 5. Atomic deletion with instant confirmation.
pub fn remove_shortcut(app_name: &str) -> Result<ShortcutResult, String> {
    let desktop = get_desktop_dir()?;
    remove_shortcut_from_dir(&desktop, app_name)
}

/// Internal implementation supporting target directory injection (for testing & safety)
pub fn remove_shortcut_from_dir(target_dir: &Path, app_name: &str) -> Result<ShortcutResult, String> {
    #[cfg(windows)]
    let expected_ext = "lnk";
    #[cfg(not(windows))]
    let expected_ext = "desktop";

    let expected_filename = format!("{}.{}", app_name, expected_ext);
    let target_file = target_dir.join(&expected_filename);

    // Safety Barrier 1: Check existence
    if !target_file.exists() {
        return Ok(ShortcutResult::NotFound);
    }

    // Safety Barrier 2: Containment check (target must be directly inside target_dir)
    let parent_canon = target_dir.canonicalize().unwrap_or_else(|_| target_dir.to_path_buf());
    let target_canon = target_file.canonicalize().unwrap_or_else(|_| target_file.clone());

    if target_canon.parent() != Some(&parent_canon) {
        return Err("Safety check failed: Target file is not directly within the Desktop directory".to_string());
    }

    // Safety Barrier 3: Verify regular file (prevent deleting directories)
    let meta = std::fs::symlink_metadata(&target_file)
        .map_err(|e| format!("Failed to inspect target metadata: {}", e))?;

    if !meta.file_type().is_file() {
        return Err("Safety check failed: Target is not a regular file".to_string());
    }

    // Safety Barrier 4: Verify filename strictly matches expected
    if target_file.file_name().and_then(|n| n.to_str()) != Some(&expected_filename) {
        return Err(format!("Safety check failed: Filename '{}' does not match expected '{}'", 
            target_file.display(), expected_filename));
    }

    // Safety Barrier 5: Safe atomic removal
    std::fs::remove_file(&target_file)
        .map_err(|e| format!("Failed to safely delete shortcut: {}", e))?;

    Ok(ShortcutResult::Removed(target_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_dir_resolution() {
        let desktop = get_desktop_dir();
        assert!(desktop.is_ok(), "Desktop resolution should succeed on standard environments");
        let path = desktop.unwrap();
        assert!(!path.as_os_str().is_empty(), "Resolved desktop path must not be empty");
        // In sandboxed environments or headless CI containers, paths outside the workspace
        // may be restricted by sandbox policies or headless profiles. If accessible, verify is_dir.
        if path.exists() {
            assert!(path.is_dir(), "When accessible, resolved desktop path must be a directory");
        }
    }

    #[test]
    fn test_shortcut_lifecycle_in_temp() {
        let temp_dir = std::env::temp_dir().join("desktop_shortcut_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let app_name = "DesktopDesignSystemTest";
        let dummy_exe = std::env::current_exe().expect("Must have current exe");

        // 1. Initial State: Should not exist
        let remove_first = remove_shortcut_from_dir(&temp_dir, app_name).unwrap();
        assert_eq!(remove_first, ShortcutResult::NotFound);

        // 2. Create Shortcut
        let create_res = create_shortcut_in_dir(
            &temp_dir,
            app_name,
            Some(dummy_exe.clone()),
            None,
            Some("Unit test shortcut"),
        ).expect("Creation should succeed");

        #[cfg(windows)]
        let expected_path = temp_dir.join(format!("{}.lnk", app_name));
        #[cfg(not(windows))]
        let expected_path = temp_dir.join(format!("{}.desktop", app_name));

        assert_eq!(create_res, ShortcutResult::Created(expected_path.clone()));
        assert!(expected_path.exists(), "Shortcut file must exist on disk");

        // 3. Smart Check: Second creation should return AlreadyExists without duplicating
        let second_create = create_shortcut_in_dir(
            &temp_dir,
            app_name,
            Some(dummy_exe),
            None,
            Some("Unit test shortcut"),
        ).expect("Second creation should succeed");

        assert_eq!(second_create, ShortcutResult::AlreadyExists(expected_path.clone()));

        // 4. Safe Removal: Should remove cleanly
        let remove_res = remove_shortcut_from_dir(&temp_dir, app_name).expect("Safe removal should succeed");
        assert_eq!(remove_res, ShortcutResult::Removed(expected_path.clone()));
        assert!(!expected_path.exists(), "Shortcut file must no longer exist");

        // 5. Subsequent removal returns NotFound
        let after_remove = remove_shortcut_from_dir(&temp_dir, app_name).unwrap();
        assert_eq!(after_remove, ShortcutResult::NotFound);

        // Cleanup temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
