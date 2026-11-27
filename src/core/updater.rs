use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::meta::{REPO, VERSION};

pub const MAX_DOWNLOAD_BYTES: u64 = 300 * 1024 * 1024;

pub fn api_latest_url() -> String {
    format!("https://api.github.com/repos/{REPO}/releases/latest")
}

pub fn releases_page_url() -> String {
    format!("https://github.com/{REPO}/releases/latest")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag: String,
    pub version: (u32, u32, u32),
    pub title: String,
    pub page_url: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

impl ReleaseInfo {
    pub fn version_text(&self) -> String {
        format_version(self.version)
    }

    pub fn asset_for_platform(&self) -> Option<&Asset> {
        let wanted = platform_asset_name();
        self.assets.iter().find(|a| a.name == wanted)
    }
}

pub fn platform_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ClickForge-Windows.zip"
    } else if cfg!(target_os = "macos") {
        "ClickForge-macOS.zip"
    } else {
        "ClickForge-Linux.zip"
    }
}

pub fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let trimmed = text.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        return None;
    }
    let prefix: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = prefix.split('.').filter(|p| !p.is_empty());
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = match parts.next() {
        Some(s) => s.parse::<u32>().ok()?,
        None => 0,
    };
    let patch = match parts.next() {
        Some(s) => s.parse::<u32>().ok()?,
        None => 0,
    };
    Some((major, minor, patch))
}

pub fn current_version() -> (u32, u32, u32) {
    parse_version(VERSION).unwrap_or((0, 0, 0))
}

pub fn format_version(v: (u32, u32, u32)) -> String {
    format!("{}.{}.{}", v.0, v.1, v.2)
}

pub fn is_newer(remote: (u32, u32, u32), local: Option<(u32, u32, u32)>) -> bool {
    remote > local.unwrap_or_else(current_version)
}

pub fn format_release_notes(notes: &str) -> String {
    let mut lines = Vec::new();
    for raw in notes.lines() {
        let s = raw.trim();
        if s.is_empty() {
            if lines.last().is_some_and(|l: &String| !l.is_empty()) {
                lines.push(String::new());
            }
            continue;
        }
        if let Some(heading) = s.strip_prefix("### ") {
            lines.push(heading.replace("**", ""));
            continue;
        }
        if let Some(heading) = s.strip_prefix("## ") {
            lines.push(heading.replace("**", ""));
            continue;
        }
        let cleaned = if let Some(rest) = s.strip_prefix("- ").or_else(|| s.strip_prefix("* ")) {
            format!("• {rest}")
        } else {
            s.to_string()
        };
        lines.push(cleaned.replace("**", "").replace('`', ""));
    }
    let joined = lines.join("\n").trim().to_string();
    if joined.is_empty() {
        "Performance optimizations, bug fixes, and stability improvements.".to_string()
    } else {
        joined
    }
}

pub fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .user_agent(&format!("ClickForge/{VERSION}"))
        .build()
}

pub fn fetch_latest() -> Result<ReleaseInfo, String> {
    let resp = http_agent()
        .get(&api_latest_url())
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Could not check GitHub Releases: {e}"))?;

    let body = resp
        .into_string()
        .map_err(|e| format!("Failed reading GitHub response: {e}"))?;
    let val: Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid GitHub response: {e}"))?;

    let tag = val
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let version = parse_version(&tag)
        .ok_or_else(|| format!("Unexpected release tag format: {tag}"))?;
    let title = val
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&tag)
        .to_string();
    let page_url = val
        .get("html_url")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .unwrap_or_else(releases_page_url);
    let notes = val
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let mut assets = Vec::new();
    if let Some(arr) = val.get("assets").and_then(Value::as_array) {
        for item in arr {
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let url = item
                .get("browser_download_url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let size = item.get("size").and_then(Value::as_u64).unwrap_or(0);
            let sha256 = item
                .get("digest")
                .and_then(Value::as_str)
                .and_then(|d| d.strip_prefix("sha256:"))
                .map(|s| s.to_ascii_lowercase());
            if !name.is_empty() && url.starts_with("https://") {
                assets.push(Asset {
                    name,
                    url,
                    size,
                    sha256,
                });
            }
        }
    }

    Ok(ReleaseInfo {
        tag,
        version,
        title,
        page_url,
        notes,
        assets,
    })
}

pub fn download_asset<F>(
    asset: &Asset,
    dest_dir: &Path,
    mut on_progress: F,
    cancel: &AtomicBool,
) -> Result<PathBuf, String>
where
    F: FnMut(u64, u64),
{
    if !asset.url.starts_with("https://") {
        return Err("Refusing non-HTTPS update URL.".to_string());
    }
    if asset.size > MAX_DOWNLOAD_BYTES {
        return Err("Update archive exceeds maximum allowed size.".to_string());
    }

    fs::create_dir_all(dest_dir).map_err(|e| format!("Failed creating temp dir: {e}"))?;
    let safe_name = Path::new(&asset.name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("update.bin");
    let target = dest_dir.join(safe_name);

    let resp = http_agent()
        .get(&asset.url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let total = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(asset.size);

    let mut reader = resp.into_reader();
    let mut file = File::create(&target).map_err(|e| format!("File creation failed: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut received: u64 = 0;

    loop {
        if cancel.load(Ordering::Acquire) {
            drop(file);
            let _ = fs::remove_file(&target);
            return Err("Download cancelled.".to_string());
        }
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Stream read failed: {e}"))?;
        if n == 0 {
            break;
        }
        received += n as u64;
        if received > MAX_DOWNLOAD_BYTES {
            drop(file);
            let _ = fs::remove_file(&target);
            return Err("Download exceeded size cap.".to_string());
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Write error: {e}"))?;
        hasher.update(&buf[..n]);
        on_progress(received, total);
    }

    if asset.size > 0 && received != asset.size {
        let _ = fs::remove_file(&target);
        return Err(format!(
            "Incomplete download ({received} of {} bytes).",
            asset.size
        ));
    }

    if let Some(expected) = &asset.sha256 {
        let digest = format!("{:x}", hasher.finalize());
        if !digest.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&target);
            return Err("Checksum mismatch; downloaded file discarded.".to_string());
        }
    }

    Ok(target)
}

pub fn safe_extract(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Could not open archive: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|_| "Downloaded archive is corrupted.".to_string())?;
    fs::create_dir_all(dest_dir).map_err(|e| format!("Failed creating extract dir: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|_| "Corrupted archive entry.".to_string())?;
        let rel = entry
            .enclosed_name()
            .ok_or_else(|| "Archive contains an unsafe path.".to_string())?
            .to_path_buf();
        if rel.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
            return Err("Archive contains an unsafe path.".to_string());
        }
        let out_path = dest_dir.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Find folder containing standalone `ClickForge.exe` (max depth 2).
pub fn find_app_root(extracted: &Path) -> Option<PathBuf> {
    if extracted.join("ClickForge.exe").is_file() {
        return Some(extracted.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(extracted) else {
        return None;
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for d in dirs {
        if d.join("ClickForge.exe").is_file() {
            return Some(d);
        }
    }
    None
}

pub fn can_self_update() -> (bool, String) {
    if !cfg!(target_os = "windows") {
        return (
            false,
            "In-place updates are supported on Windows; opening release page.".to_string(),
        );
    }
    let Ok(exe) = std::env::current_exe() else {
        return (false, "Cannot locate running executable.".to_string());
    };
    let Some(dir) = exe.parent() else {
        return (false, "Cannot locate application directory.".to_string());
    };
    // Skip dev builds inside Cargo's target directory
    if dir
        .components()
        .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("target"))
    {
        return (
            false,
            "Development build detected; opening the download page.".to_string(),
        );
    }
    let probe = dir.join(".cf-write-test.tmp");
    match File::create(&probe) {
        Ok(f) => {
            drop(f);
            let _ = fs::remove_file(&probe);
            (true, String::new())
        }
        Err(_) => (
            false,
            "ClickForge's folder is read-only. Download the update manually.".to_string(),
        ),
    }
}

pub fn stage_update(zip_path: &Path, workdir: &Path) -> Result<PathBuf, String> {
    let staged = workdir.join("staged");
    let _ = fs::remove_dir_all(&staged);
    safe_extract(zip_path, &staged)?;
    find_app_root(&staged).ok_or_else(|| "Archive does not contain ClickForge.exe.".to_string())
}

#[cfg(target_os = "windows")]
const APPLY_UPDATE_CMD: &str = r#"@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "LOG=%CF_WORK%\update.log"
echo ClickForge updater started > "%LOG%"
set /a TRIES=0

:wait
tasklist /FI "PID eq %CF_PID%" /FO CSV /NH 2>nul | findstr /R /C:",.%CF_PID%.," >nul
if errorlevel 1 goto swap
set /a TRIES+=1
if %TRIES% GEQ 90 goto give_up
ping -n 2 127.0.0.1 >nul
goto wait

:swap
echo Replacing %CF_DST%\%CF_EXE% >> "%LOG%"
if exist "%CF_DST%\%CF_EXE%.cfbak" del /f /q "%CF_DST%\%CF_EXE%.cfbak" >nul 2>&1
copy /y "%CF_DST%\%CF_EXE%" "%CF_DST%\%CF_EXE%.cfbak" >> "%LOG%" 2>&1

set /a COPY_TRIES=0
:copy_new
copy /y "%CF_SRC%\ClickForge.exe" "%CF_DST%\%CF_EXE%" >> "%LOG%" 2>&1
if not errorlevel 1 goto swapped
set /a COPY_TRIES+=1
if %COPY_TRIES% GEQ 6 goto rollback
ping -n 2 127.0.0.1 >nul
goto copy_new

:swapped
if exist "%CF_DST%\_internal" rmdir /s /q "%CF_DST%\_internal" >nul 2>&1
del /f /q "%CF_DST%\%CF_EXE%.cfbak" >nul 2>&1
echo Update installed >> "%LOG%"
start "" /D "%CF_DST%" "%CF_DST%\%CF_EXE%"
rmdir /s /q "%CF_WORK%\staged" >nul 2>&1
del /f /q "%CF_WORK%\*.zip" >nul 2>&1
goto done

:rollback
echo Install failed, rolling back >> "%LOG%"
copy /y "%CF_DST%\%CF_EXE%.cfbak" "%CF_DST%\%CF_EXE%" >> "%LOG%" 2>&1
del /f /q "%CF_DST%\%CF_EXE%.cfbak" >nul 2>&1

:give_up
echo Update aborted >> "%LOG%"
start "" /D "%CF_DST%" "%CF_DST%\%CF_EXE%"

:done
endlocal
(goto) 2>nul & del "%~f0"
"#;

pub fn launch_self_updater(staged_app: &Path, workdir: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;

        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let dst = exe.parent().ok_or("Missing install directory")?;
        let exe_name = exe
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("ClickForge.exe");

        let script = workdir.join("apply_update.cmd");
        fs::write(&script, APPLY_UPDATE_CMD.replace('\n', "\r\n"))
            .map_err(|e| format!("Failed writing updater helper: {e}"))?;

        let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
        Command::new(comspec)
            .args(["/c", &script.to_string_lossy()])
            .env("CF_PID", std::process::id().to_string())
            .env("CF_SRC", staged_app.to_string_lossy().as_ref())
            .env("CF_DST", dst.to_string_lossy().as_ref())
            .env("CF_EXE", exe_name)
            .env("CF_WORK", workdir.to_string_lossy().as_ref())
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed starting updater helper: {e}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (staged_app, workdir);
        Err("In-place updates are only supported on Windows.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parsing_and_comparison() {
        assert_eq!(parse_version("v4.0.0"), Some((4, 0, 0)));
        assert_eq!(parse_version("3.3.2-beta"), Some((3, 3, 2)));
        assert_eq!(parse_version("invalid"), None);
        assert!(is_newer((4, 1, 0), Some((4, 0, 0))));
        assert!(!is_newer((4, 0, 0), Some((4, 0, 0))));
        assert!(!is_newer((3, 9, 9), Some((4, 0, 0))));
    }

    #[test]
    fn zipslip_rejected_and_standalone_exe_found() {
        let dir = std::env::temp_dir().join("clickforge_test_zip_safety");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let bad_zip = dir.join("evil.zip");
        {
            let f = File::create(&bad_zip).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("../escape.txt", opts).unwrap();
            zw.write_all(b"pwn").unwrap();
            zw.finish().unwrap();
        }
        assert!(safe_extract(&bad_zip, &dir.join("out_bad")).is_err());

        let good_zip = dir.join("good.zip");
        {
            let f = File::create(&good_zip).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("ClickForge/ClickForge.exe", opts).unwrap();
            zw.write_all(b"MZ").unwrap();
            zw.finish().unwrap();
        }
        let app_root = stage_update(&good_zip, &dir).unwrap();
        assert!(app_root.join("ClickForge.exe").is_file());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_notes_formatting() {
        let md = "### Highlights\n- **Fast:** `1,000 CPS`\n\n";
        let clean = format_release_notes(md);
        assert_eq!(clean, "Highlights\n• Fast: 1,000 CPS");
    }
}
