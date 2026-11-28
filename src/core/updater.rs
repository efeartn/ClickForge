use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::meta::{APP_EXE, APP_NAME, REPO, VERSION};

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
        "Klick-Windows.zip"
    } else if cfg!(target_os = "macos") {
        "Klick-macOS.zip"
    } else {
        "Klick-Linux.zip"
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
        .user_agent(&format!("{APP_NAME}/{VERSION}"))
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
        .set("Accept-Encoding", "identity")
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

/// Find folder containing standalone `Klick.exe` (max depth 2).
pub fn find_app_root(extracted: &Path) -> Option<PathBuf> {
    if extracted.join(APP_EXE).is_file() {
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
    dirs.into_iter().find(|d| d.join(APP_EXE).is_file())
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
    let probe = dir.join(".klick-write-test.tmp");
    match File::create(&probe) {
        Ok(f) => {
            drop(f);
            let _ = fs::remove_file(&probe);
            (true, String::new())
        }
        Err(_) => (
            false,
            format!("{APP_NAME}'s folder is read-only. Download the update manually."),
        ),
    }
}

pub fn stage_update(zip_path: &Path, workdir: &Path) -> Result<PathBuf, String> {
    let staged = workdir.join("staged");
    let _ = fs::remove_dir_all(&staged);
    safe_extract(zip_path, &staged)?;
    find_app_root(&staged).ok_or_else(|| format!("Archive does not contain {APP_EXE}."))
}

#[cfg(target_os = "windows")]
const APPLY_UPDATE_PS1: &str = r#"$ErrorActionPreference = 'Stop'
$cfPid = [int]$env:CF_PID
$srcDir = $env:CF_SRC
$dstDir = $env:CF_DST
$exeName = $env:CF_EXE
$workDir = $env:CF_WORK
$newVer = $env:CF_VER
$noRelaunch = ($env:CF_NO_RELAUNCH -eq '1')
$log = [System.IO.Path]::Combine($workDir, 'update.log')
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Write-Log([string]$msg) {
    $ts = (Get-Date).ToString('HH:mm:ss.fff')
    try {
        [System.IO.File]::AppendAllText($log, "[$ts] $msg`r`n", $utf8NoBom)
    } catch {}
}

function Get-Sha256Hex([string]$filePath) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream = [System.IO.File]::OpenRead($filePath)
        try {
            $hash = $sha.ComputeHash($stream)
            return ([System.BitConverter]::ToString($hash)).Replace('-', '').ToLowerInvariant()
        } finally {
            $stream.Dispose()
        }
    } finally {
        $sha.Dispose()
    }
}

[System.IO.File]::WriteAllText($log, "Klick updater started`r`n", $utf8NoBom)

if ($cfPid -gt 0) {
    try {
        $proc = [System.Diagnostics.Process]::GetProcessById($cfPid)
        Write-Log "Waiting for PID $cfPid to exit..."
        if (-not $proc.WaitForExit(30000)) {
            Write-Log "PID $cfPid did not exit in 30s; forcing stop"
            try { $proc.Kill() } catch {}
            $proc.WaitForExit(5000) | Out-Null
        }
    } catch {
        Write-Log "PID $cfPid already exited"
    }
}

$dstExe = [System.IO.Path]::Combine($dstDir, $exeName)
$srcExe = [System.IO.Path]::Combine($srcDir, 'Klick.exe')
$bakExe = "$dstExe.klbak"

$stem = [System.IO.Path]::GetFileNameWithoutExtension($exeName)
try {
    foreach ($p in [System.Diagnostics.Process]::GetProcessesByName($stem)) {
        try {
            if ($p.Id -ne $cfPid -and $p.MainModule.FileName -ieq $dstExe) {
                Write-Log "Closing running instance PID $($p.Id)"
                $p.CloseMainWindow() | Out-Null
                if (-not $p.WaitForExit(3000)) { $p.Kill() }
            }
        } catch {}
    }
} catch {}

$expectedHash = Get-Sha256Hex $srcExe
$swapped = $false

for ($try = 1; $try -le 15; $try++) {
    try {
        if ([System.IO.File]::Exists($bakExe)) {
            [System.IO.File]::Delete($bakExe)
        }
        if ([System.IO.File]::Exists($dstExe)) {
            [System.IO.File]::Move($dstExe, $bakExe)
        }
        [System.IO.File]::Copy($srcExe, $dstExe, $true)
        $actualHash = Get-Sha256Hex $dstExe
        if ($actualHash -ne $expectedHash) {
            throw "SHA-256 mismatch ($actualHash != $expectedHash)"
        }
        $swapped = $true
        break
    } catch {
        Write-Log "Attempt $try failed: $($_.Exception.Message)"
        if ([System.IO.File]::Exists($bakExe)) {
            try { [System.IO.File]::Copy($bakExe, $dstExe, $true) } catch {}
        }
        [System.Threading.Thread]::Sleep(300)
    }
}

if ($swapped) {
    $legacyInternal = [System.IO.Path]::Combine($dstDir, '_internal')
    if ([System.IO.Directory]::Exists($legacyInternal)) {
        try { [System.IO.Directory]::Delete($legacyInternal, $true) } catch {}
    }
    if ([System.IO.File]::Exists($bakExe)) {
        try { [System.IO.File]::Delete($bakExe) } catch {}
    }
    if ($newVer) {
        try {
            $regKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Uninstall\Klick', $true)
            if ($regKey) {
                $regKey.SetValue('DisplayVersion', $newVer)
                $regKey.Close()
            }
        } catch {}
    }
    Write-Log 'Update installed'
    if (-not $noRelaunch) {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $dstExe
        $psi.WorkingDirectory = $dstDir
        $psi.UseShellExecute = $true
        [System.Diagnostics.Process]::Start($psi) | Out-Null
    }
    $stagedDir = [System.IO.Path]::Combine($workDir, 'staged')
    if ([System.IO.Directory]::Exists($stagedDir)) {
        try { [System.IO.Directory]::Delete($stagedDir, $true) } catch {}
    }
    try {
        foreach ($zf in [System.IO.Directory]::GetFiles($workDir, '*.zip')) {
            try { [System.IO.File]::Delete($zf) } catch {}
        }
    } catch {}
} else {
    Write-Log 'Update aborted; restoring backup'
    if ([System.IO.File]::Exists($bakExe)) {
        try { [System.IO.File]::Copy($bakExe, $dstExe, $true) } catch {}
        try { [System.IO.File]::Delete($bakExe) } catch {}
    }
    if (-not $noRelaunch -and [System.IO.File]::Exists($dstExe)) {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $dstExe
        $psi.WorkingDirectory = $dstDir
        $psi.UseShellExecute = $true
        [System.Diagnostics.Process]::Start($psi) | Out-Null
    }
}

$self = $MyInvocation.MyCommand.Path
if ($self -and [System.IO.File]::Exists($self)) {
    try { [System.IO.File]::Delete($self) } catch {}
}
if ($swapped) { exit 0 } else { exit 1 }
"#;

pub fn launch_self_updater(staged_app: &Path, workdir: &Path, new_version: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let dst = exe.parent().ok_or("Missing install directory")?;
        let exe_name = exe
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(APP_EXE);

        let script = workdir.join("apply_update.ps1");
        let normalized = APPLY_UPDATE_PS1.replace("\r\n", "\n").replace('\n', "\r\n");
        fs::write(&script, normalized)
            .map_err(|e| format!("Failed writing updater helper: {e}"))?;

        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script.to_string_lossy(),
            ])
            .env("CF_PID", std::process::id().to_string())
            .env("CF_SRC", staged_app.to_string_lossy().as_ref())
            .env("CF_DST", dst.to_string_lossy().as_ref())
            .env("CF_EXE", exe_name)
            .env("CF_WORK", workdir.to_string_lossy().as_ref())
            .env("CF_VER", new_version)
            .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed starting updater helper: {e}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (staged_app, workdir, new_version);
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
        let dir = std::env::temp_dir().join("klick_test_zip_safety");
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
            zw.start_file("Klick/Klick.exe", opts).unwrap();
            zw.write_all(b"MZ").unwrap();
            zw.finish().unwrap();
        }
        let app_root = stage_update(&good_zip, &dir).unwrap();
        assert!(app_root.join(APP_EXE).is_file());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_notes_formatting() {
        let md = "### Highlights\n- **Fast:** `1,000 CPS`\n\n";
        let clean = format_release_notes(md);
        assert_eq!(clean, "Highlights\n• Fast: 1,000 CPS");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_self_updater_swaps_and_verifies_exe() {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        let root = std::env::temp_dir().join("klick_test_ps1_swap");
        let _ = fs::remove_dir_all(&root);
        let dst = root.join("dst");
        let work = root.join("work");
        let src = work.join("staged");
        let legacy_internal = dst.join("_internal");
        fs::create_dir_all(&legacy_internal).unwrap();
        fs::create_dir_all(&src).unwrap();

        let dst_exe = dst.join(APP_EXE);
        let src_exe = src.join(APP_EXE);
        fs::write(&dst_exe, b"OLD_BINARY_PAYLOAD").unwrap();
        fs::write(legacy_internal.join("stale.dll"), b"OLD_RUNTIME").unwrap();
        fs::write(&src_exe, b"NEW_NATIVE_RUST_BINARY_V1_0_0").unwrap();

        let script = work.join("apply_update.ps1");
        let normalized = APPLY_UPDATE_PS1.replace("\r\n", "\n").replace('\n', "\r\n");
        fs::write(&script, normalized).unwrap();

        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script.to_string_lossy(),
            ])
            .env("CF_PID", "999999")
            .env("CF_SRC", src.to_string_lossy().as_ref())
            .env("CF_DST", dst.to_string_lossy().as_ref())
            .env("CF_EXE", APP_EXE)
            .env("CF_WORK", work.to_string_lossy().as_ref())
            .env("CF_VER", "")
            .env("CF_NO_RELAUNCH", "1")
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .output()
            .expect("powershell updater failed to launch");

        let log = fs::read_to_string(work.join("update.log")).unwrap_or_default();
        assert!(
            output.status.success(),
            "ps1 exited {:?}; stdout={}; stderr={}; log={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            log
        );
        assert_eq!(fs::read(&dst_exe).unwrap(), b"NEW_NATIVE_RUST_BINARY_V1_0_0");
        assert!(!dst.join("Klick.exe.klbak").exists());
        assert!(!legacy_internal.exists());
        assert!(!src.exists());
        assert!(log.contains("Update installed"), "unexpected log: {log}");

        let _ = fs::remove_dir_all(&root);
    }
}
