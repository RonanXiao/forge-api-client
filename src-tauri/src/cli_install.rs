//! Install / uninstall the bundled `forge-cli` onto the user PATH via symlink.
//! The binary lives inside Forge.app; install only creates a reference (no copy).

use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

/// Preferred link name on PATH (`forge` command).
const LINK_NAME: &str = "forge";

/// Bundled binary name inside Resources/bin/
const BUNDLED_NAME: &str = "forge-cli";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallStatus {
    pub installed: bool,
    pub link_path: String,
    pub target_path: Option<String>,
    pub bundled_path: Option<String>,
    pub message: String,
}

fn user_bin_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not resolve home directory")?;
    // ~/.local/bin — no admin rights required
    Ok(home.join(".local").join("bin"))
}

fn link_path() -> Result<PathBuf, String> {
    Ok(user_bin_dir()?.join(LINK_NAME))
}

/// Locate forge-cli next to the app / in Resources/bin.
pub fn bundled_cli_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    // 1) resource_dir/bin/forge-cli (release bundle)
    if let Ok(res) = app.path().resource_dir() {
        let p = res.join("bin").join(BUNDLED_NAME);
        if p.is_file() {
            return Ok(p);
        }
        // Some bundlers put resources at resource_dir root
        let p2 = res.join(BUNDLED_NAME);
        if p2.is_file() {
            return Ok(p2);
        }
    }

    // 2) Same directory as the GUI executable (dev / ad-hoc)
    if let Ok(exe) = app.path().executable_dir() {
        let p = exe.join(BUNDLED_NAME);
        if p.is_file() {
            return Ok(p);
        }
        // target/release sibling when running from cargo
        if let Some(parent) = exe.parent() {
            let p = parent.join(BUNDLED_NAME);
            if p.is_file() {
                return Ok(p);
            }
        }
    }

    // 3) current_exe parent
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(BUNDLED_NAME);
            if p.is_file() {
                return Ok(p);
            }
            // Forge.app/Contents/MacOS → ../Resources/bin/forge-cli
            if let Some(contents) = dir.parent() {
                let p = contents.join("Resources").join("bin").join(BUNDLED_NAME);
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
    }

    Err(
        "Bundled forge-cli not found. Build a release app (pnpm tauri:build) \
or ensure forge-cli sits next to the Forge binary."
            .into(),
    )
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> CliInstallStatus {
    let link = link_path().ok();
    let bundled = bundled_cli_path(app).ok();
    let link_path = link
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.local/bin/forge".into());

    let (installed, target_path) = if let Some(ref lp) = link {
        if lp.exists() || lp.symlink_metadata().is_ok() {
            let target = fs::read_link(lp)
                .ok()
                .map(|t| t.display().to_string())
                .or_else(|| Some(lp.display().to_string()));
            (true, target)
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    let message = if installed {
        format!("forge-cli is installed at {link_path}")
    } else {
        "forge-cli is not installed on PATH".into()
    };

    CliInstallStatus {
        installed,
        link_path,
        target_path,
        bundled_path: bundled.map(|p| p.display().to_string()),
        message,
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> Result<CliInstallStatus, String> {
    let target = bundled_cli_path(app)?;
    let bin_dir = user_bin_dir()?;
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Create ~/.local/bin failed: {e}"))?;
    let link = bin_dir.join(LINK_NAME);

    // Remove existing link/file if present
    if link.symlink_metadata().is_ok() {
        fs::remove_file(&link).map_err(|e| format!("Remove old link failed: {e}"))?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, &link)
            .map_err(|e| format!("Create symlink failed: {e}"))?;
    }
    #[cfg(windows)]
    {
        // Windows: try symlink; fall back to copy if policy blocks
        if std::os::windows::fs::symlink_file(&target, &link).is_err() {
            fs::copy(&target, &link).map_err(|e| format!("Copy CLI failed: {e}"))?;
        }
    }

    // Ensure target is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&target) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o755);
            let _ = fs::set_permissions(&target, perms);
        }
    }

    let mut st = status(app);
    st.message = format!(
        "Installed. Symlink: {} → {}\n\n\
If `forge` is not found, add to your shell profile:\n  export PATH=\"$HOME/.local/bin:$PATH\"\n\n\
Try: forge --help",
        link.display(),
        target.display()
    );
    Ok(st)
}

pub fn uninstall<R: Runtime>(app: &AppHandle<R>) -> Result<CliInstallStatus, String> {
    let link = link_path()?;
    if link.symlink_metadata().is_ok() {
        fs::remove_file(&link).map_err(|e| format!("Remove link failed: {e}"))?;
    }
    let mut st = status(app);
    st.message = format!("Uninstalled. Removed {}", link.display());
    Ok(st)
}


