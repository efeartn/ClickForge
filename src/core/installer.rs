use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::core::updater::{self, Asset, ReleaseInfo};
use crate::meta::APP_NAME;

pub const SETUP_ASSET_NAME: &str = "ClickForge-Setup.exe";

pub fn default_install_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData").join("Local"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Programs").join(APP_NAME)
}

pub fn is_installed_in_programs(exe_override: Option<&Path>) -> bool {
    if !cfg!(target_os = "windows") {
        return true;
    }
    let exe = match exe_override {
        Some(p) => p.to_path_buf(),
        None => match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return false,
        },
    };
    let mut candidates = vec![default_install_dir()];
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(var) {
            candidates.push(PathBuf::from(root).join(APP_NAME));
        }
    }
    candidates
        .iter()
        .any(|dir| path_starts_with_ci(&exe, dir))
}

fn path_starts_with_ci(child: &Path, parent: &Path) -> bool {
    let c = child.to_string_lossy().to_ascii_lowercase();
    let p = parent.to_string_lossy().to_ascii_lowercase();
    c == p || c.starts_with(&(p + std::path::MAIN_SEPARATOR_STR))
}

pub fn can_install_portable() -> bool {
    if !cfg!(target_os = "windows") || is_installed_in_programs(None) {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    !exe.components()
        .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("target"))
}

pub fn find_local_installer() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(SETUP_ASSET_NAME);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn select_setup_asset(release: &ReleaseInfo) -> Option<&Asset> {
    release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(SETUP_ASSET_NAME))
}

pub fn launch_exe(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::{Command, Stdio};
        let mut cmd = Command::new(path);
        if let Some(dir) = path.parent() {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed launching {}: {e}", path.display()))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("Windows installer is only supported on Windows.".to_string())
    }
}

pub fn download_and_launch_setup<F>(
    on_progress: F,
    cancel: &AtomicBool,
) -> Result<PathBuf, String>
where
    F: FnMut(u64, u64),
{
    let release = updater::fetch_latest()?;
    let asset = select_setup_asset(&release)
        .ok_or_else(|| "Release does not include ClickForge-Setup.exe.".to_string())?;
    let workdir = std::env::temp_dir().join("clickforge-setup");
    let setup_path = updater::download_asset(asset, &workdir, on_progress, cancel)?;
    launch_exe(&setup_path)?;
    Ok(setup_path)
}

pub fn install_portable_to_programs(create_desktop_shortcut: bool) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        use std::fs;
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        use crate::meta::VERSION;

        let src_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let install_dir = default_install_dir();
        fs::create_dir_all(&install_dir)
            .map_err(|e| format!("Failed creating install dir: {e}"))?;

        let dst_exe = install_dir.join("ClickForge.exe");
        fs::copy(&src_exe, &dst_exe).map_err(|e| format!("Failed copying ClickForge.exe: {e}"))?;

        let stale_internal = install_dir.join("_internal");
        if stale_internal.exists() {
            let _ = fs::remove_dir_all(&stale_internal);
        }

        let size_kb = fs::metadata(&dst_exe)
            .map(|m| (m.len() / 1024).max(1) as u32)
            .unwrap_or(4096);

        write_uninstall_registry(&install_dir, &dst_exe, VERSION, size_kb)?;

        let start_menu = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Microsoft\\Windows\\Start Menu\\Programs")
            .join(APP_NAME);
        let _ = fs::create_dir_all(&start_menu);
        let start_lnk = start_menu.join(format!("{APP_NAME}.lnk"));

        let mut ps = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $s = $ws.CreateShortcut('{}'); \
             $s.TargetPath = '{}'; \
             $s.WorkingDirectory = '{}'; \
             $s.IconLocation = '{},0'; \
             $s.Save(); ",
            ps_escape(&start_lnk),
            ps_escape(&dst_exe),
            ps_escape(&install_dir),
            ps_escape(&dst_exe),
        );
        if create_desktop_shortcut {
            if let Some(profile) = std::env::var_os("USERPROFILE") {
                let desktop_lnk = PathBuf::from(profile)
                    .join("Desktop")
                    .join(format!("{APP_NAME}.lnk"));
                ps.push_str(&format!(
                    "$d = $ws.CreateShortcut('{}'); \
                     $d.TargetPath = '{}'; \
                     $d.WorkingDirectory = '{}'; \
                     $d.IconLocation = '{},0'; \
                     $d.Save();",
                    ps_escape(&desktop_lnk),
                    ps_escape(&dst_exe),
                    ps_escape(&install_dir),
                    ps_escape(&dst_exe),
                ));
            }
        }
        let _ = Command::new("powershell.exe")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps])
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        Ok(dst_exe)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = create_desktop_shortcut;
        Err("Windows Programs installation is only supported on Windows.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn ps_escape(p: &Path) -> String {
    p.to_string_lossy().replace('\'', "''")
}

#[cfg(target_os = "windows")]
fn write_uninstall_registry(
    install_dir: &Path,
    dst_exe: &Path,
    version: &str,
    size_kb: u32,
) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, REG_DWORD, REG_SZ,
    };

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\ClickForge");
    let mut hkey: HKEY = std::ptr::null_mut();
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey.as_ptr(), &mut hkey) };
    if status != 0 {
        return Err(format!("Failed writing uninstall registry key (err {status})."));
    }

    let set_str = |name: &str, val: &str| unsafe {
        let k = wide(name);
        let v = wide(val);
        RegSetValueExW(
            hkey,
            k.as_ptr(),
            0,
            REG_SZ,
            v.as_ptr() as *const u8,
            (v.len() * 2) as u32,
        );
    };
    let set_dword = |name: &str, val: u32| unsafe {
        let k = wide(name);
        let bytes = val.to_le_bytes();
        RegSetValueExW(hkey, k.as_ptr(), 0, REG_DWORD, bytes.as_ptr(), 4);
    };

    let dst_s = dst_exe.to_string_lossy();
    let dir_s = install_dir.to_string_lossy();
    set_str("DisplayName", APP_NAME);
    set_str("DisplayVersion", version);
    set_str("Publisher", "efeartn");
    set_str("InstallLocation", &dir_s);
    set_str("DisplayIcon", &format!("{dst_s},0"));
    set_str(
        "UninstallString",
        &format!("cmd.exe /c rmdir /s /q \"{dir_s}\""),
    );
    set_dword("EstimatedSize", size_kb);
    set_dword("NoModify", 1);
    set_dword("NoRepair", 1);

    unsafe { RegCloseKey(hkey) };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_detection_works() {
        let target = default_install_dir().join("ClickForge.exe");
        assert!(is_installed_in_programs(Some(&target)));
        if cfg!(target_os = "windows") {
            let desktop = PathBuf::from(r"C:\Temp\ClickForge\ClickForge.exe");
            assert!(!is_installed_in_programs(Some(&desktop)));
        }
    }

    #[test]
    fn select_setup_finds_asset() {
        let rel = ReleaseInfo {
            tag: "v4.0.0".to_string(),
            version: (4, 0, 0),
            title: "v4.0.0".to_string(),
            page_url: String::new(),
            notes: String::new(),
            assets: vec![Asset {
                name: "ClickForge-Setup.exe".to_string(),
                url: "https://example.com/setup.exe".to_string(),
                size: 123,
                sha256: None,
            }],
        };
        assert!(select_setup_asset(&rel).is_some());
    }
}
