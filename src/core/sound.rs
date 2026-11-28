use std::thread;

pub fn play_cue(starting: bool, enabled: bool) {
    if !enabled {
        return;
    }
    let freq: u32 = if starting { 1320 } else { 880 };
    thread::Builder::new()
        .name("Klick-AudioCue".to_string())
        .spawn(move || play_worker(freq, 35))
        .ok();
}

#[cfg(target_os = "windows")]
fn play_worker(freq: u32, duration_ms: u32) {
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::Beep(freq, duration_ms);
    }
}

#[cfg(target_os = "macos")]
fn play_worker(freq: u32, _duration_ms: u32) {
    use std::process::{Command, Stdio};
    let sound_file = if freq > 1000 {
        "/System/Library/Sounds/Tink.aiff"
    } else {
        "/System/Library/Sounds/Pop.aiff"
    };
    let _ = Command::new("afplay")
        .arg(sound_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn play_worker(_freq: u32, _duration_ms: u32) {
    use std::path::Path;
    use std::process::{Command, Stdio};
    if Command::new("canberra-gtk-play")
        .args(["-i", "message"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return;
    }
    let sound = "/usr/share/sounds/freedesktop/stereo/message.oga";
    if Path::new(sound).is_file() {
        let _ = Command::new("paplay")
            .arg(sound)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
