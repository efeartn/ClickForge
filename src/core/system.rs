use std::fs;
use std::path::PathBuf;

/// Returns `(left, top, width, height)` in global screen points/pixels.
pub fn virtual_screen_bounds() -> Option<(i32, i32, i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };
        unsafe {
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            if w <= 0 || h <= 0 {
                return None;
            }
            return Some((
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                w,
                h,
            ));
        }
    }

    #[cfg(target_os = "macos")]
    {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct CGPoint {
            x: f64,
            y: f64,
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct CGSize {
            width: f64,
            height: f64,
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct CGRect {
            origin: CGPoint,
            size: CGSize,
        }

        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGGetActiveDisplayList(
                max_displays: u32,
                active_displays: *mut u32,
                display_count: *mut u32,
            ) -> i32;
            fn CGDisplayBounds(display: u32) -> CGRect;
        }

        unsafe {
            let mut count: u32 = 0;
            if CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut count) != 0 || count == 0 {
                return None;
            }
            let mut displays = vec![0u32; count as usize];
            if CGGetActiveDisplayList(count, displays.as_mut_ptr(), &mut count) != 0 || count == 0
            {
                return None;
            }
            let mut left = f64::INFINITY;
            let mut top = f64::INFINITY;
            let mut right = f64::NEG_INFINITY;
            let mut bottom = f64::NEG_INFINITY;
            for &d in &displays[..count as usize] {
                let r = CGDisplayBounds(d);
                left = left.min(r.origin.x);
                top = top.min(r.origin.y);
                right = right.max(r.origin.x + r.size.width);
                bottom = bottom.max(r.origin.y + r.size.height);
            }
            let w = (right - left).round() as i32;
            let h = (bottom - top).round() as i32;
            if w <= 0 || h <= 0 {
                return None;
            }
            return Some((left.round() as i32, top.round() as i32, w, h));
        }
    }

    #[cfg(target_os = "linux")]
    {
        use x11rb::connection::Connection;
        let Ok((conn, screen_num)) = x11rb::connect(None) else {
            return None;
        };
        let s = &conn.setup().roots[screen_num];
        return Some((0, 0, s.width_in_pixels as i32, s.height_in_pixels as i32));
    }
}

pub fn startup_warnings() -> Vec<String> {
    #[allow(unused_mut)]
    let mut warnings = Vec::new();
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        if !unsafe { AXIsProcessTrusted() } {
            warnings.push(
                "ClickForge needs Accessibility access for global hotkeys and mouse clicks.\nOpen System Settings -> Privacy & Security -> Accessibility and enable ClickForge."
                    .to_string(),
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        if std::env::var("XDG_SESSION_TYPE")
            .map(|s| s.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
        {
            warnings.push(
                "Wayland restricts synthetic global clicks and hotkeys; switch to an X11/Xorg session if hotkeys or clicking do not respond."
                    .to_string(),
            );
        }
    }

    warnings
}

/// Remove the temporary `_internal/.migrated` folder if we upgraded in-place from Python v3.x.
pub fn cleanup_legacy_migration_marker() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let internal = dir.join("_internal");
    if internal.is_dir() && internal.join(".migrated").is_file() {
        let _ = fs::remove_dir_all(&internal);
    }
}

#[allow(dead_code)]
pub fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}
