use serde_json::Value;

/// Map Linux `<linux/input-event-codes.h>` key/button codes into ClickForge canonical hotkey names.
#[allow(dead_code)]
pub fn evdev_code_to_name(code: u16) -> Option<String> {
    let s: &str = match code {
        1 => "esc",
        2 => "1",
        3 => "2",
        4 => "3",
        5 => "4",
        6 => "5",
        7 => "6",
        8 => "7",
        9 => "8",
        10 => "9",
        11 => "0",
        12 => "-",
        13 => "=",
        14 => "backspace",
        15 => "tab",
        16 => "q",
        17 => "w",
        18 => "e",
        19 => "r",
        20 => "t",
        21 => "y",
        22 => "u",
        23 => "i",
        24 => "o",
        25 => "p",
        26 => "[",
        27 => "]",
        28 => "enter",
        29 => "ctrl_l",
        30 => "a",
        31 => "s",
        32 => "d",
        33 => "f",
        34 => "g",
        35 => "h",
        36 => "j",
        37 => "k",
        38 => "l",
        39 => ";",
        40 => "'",
        41 => "`",
        42 => "shift",
        43 => "\\",
        44 => "z",
        45 => "x",
        46 => "c",
        47 => "v",
        48 => "b",
        49 => "n",
        50 => "m",
        51 => ",",
        52 => ".",
        53 => "/",
        54 => "shift_r",
        55 => "vk106", // Num *
        56 => "alt_l",
        57 => "space",
        58 => "caps_lock",
        59..=68 => return Some(format!("f{}", code - 58)), // F1..F10
        69 => "num_lock",
        70 => "scroll_lock",
        71 => "vk103", // Num 7
        72 => "vk104", // Num 8
        73 => "vk105", // Num 9
        74 => "vk109", // Num -
        75 => "vk100", // Num 4
        76 => "vk101", // Num 5
        77 => "vk102", // Num 6
        78 => "vk107", // Num +
        79 => "vk97",  // Num 1
        80 => "vk98",  // Num 2
        81 => "vk99",  // Num 3
        82 => "vk96",  // Num 0
        83 => "vk110", // Num .
        87 => "f11",
        88 => "f12",
        97 => "ctrl_r",
        98 => "vk111", // Num /
        99 => "print_screen",
        100 => "alt_r",
        102 => "home",
        103 => "up",
        104 => "page_up",
        105 => "left",
        106 => "right",
        107 => "end",
        108 => "down",
        109 => "page_down",
        110 => "insert",
        111 => "delete",
        113 => "media_volume_mute",
        114 => "media_volume_down",
        115 => "media_volume_up",
        119 => "pause",
        125 => "cmd",
        126 => "cmd_r",
        127 => "menu",
        163 => "media_next",
        164 => "media_play_pause",
        165 => "media_previous",
        166 => "media_stop",
        183..=194 => return Some(format!("f{}", code - 170)), // F13..F24
        0x113 => "mouse:x1", // BTN_SIDE (Mouse4)
        0x114 => "mouse:x2", // BTN_EXTRA (Mouse5)
        _ => return None,
    };
    Some(s.to_string())
}

#[allow(dead_code)]
pub fn parse_hyprland_cursorpos(raw: &str) -> Option<(i32, i32)> {
    let mut parts = raw.trim().split(',');
    let x = parts.next()?.trim().parse::<i32>().ok()?;
    let y = parts.next()?.trim().parse::<i32>().ok()?;
    Some((x, y))
}

#[allow(dead_code)]
pub fn parse_hyprland_monitors_bounds(json: &str) -> Option<(i32, i32, i32, i32)> {
    let val: Value = serde_json::from_str(json).ok()?;
    let arr = val.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;
    let mut any = false;

    for mon in arr {
        let Some(obj) = mon.as_object() else {
            continue;
        };
        if obj.get("disabled").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let x = obj.get("x").and_then(Value::as_i64).unwrap_or(0) as i32;
        let y = obj.get("y").and_then(Value::as_i64).unwrap_or(0) as i32;
        let raw_w = obj.get("width").and_then(Value::as_i64).unwrap_or(0) as f64;
        let raw_h = obj.get("height").and_then(Value::as_i64).unwrap_or(0) as f64;
        let scale = obj
            .get("scale")
            .and_then(Value::as_f64)
            .filter(|s| *s > 0.0 && s.is_finite())
            .unwrap_or(1.0);
        let transform = obj.get("transform").and_then(Value::as_i64).unwrap_or(0);
        let (lw, lh) = if matches!(transform % 4, 1 | 3) {
            ((raw_h / scale).round() as i32, (raw_w / scale).round() as i32)
        } else {
            ((raw_w / scale).round() as i32, (raw_h / scale).round() as i32)
        };
        if lw <= 0 || lh <= 0 {
            continue;
        }
        left = left.min(x);
        top = top.min(y);
        right = right.max(x + lw);
        bottom = bottom.max(y + lh);
        any = true;
    }

    if !any || right <= left || bottom <= top {
        return None;
    }
    Some((left, top, right - left, bottom - top))
}

#[cfg(target_os = "linux")]
pub mod linux_rt {
    use std::ffi::CString;
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Write};
    use std::mem::{size_of, zeroed};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::{AsRawFd, RawFd};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::time::Duration;

    use crate::core::input_backend::MouseButton;

    pub const EV_SYN: u16 = 0x00;
    pub const EV_KEY: u16 = 0x01;
    pub const EV_REL: u16 = 0x02;
    pub const EV_ABS: u16 = 0x03;

    pub const SYN_REPORT: u16 = 0x00;
    pub const REL_X: u16 = 0x00;
    pub const REL_Y: u16 = 0x01;
    pub const ABS_X: u16 = 0x00;
    pub const ABS_Y: u16 = 0x01;

    pub const BTN_LEFT: u16 = 0x110;
    pub const BTN_RIGHT: u16 = 0x111;
    pub const BTN_MIDDLE: u16 = 0x112;
    pub const BTN_SIDE: u16 = 0x113;
    pub const BTN_EXTRA: u16 = 0x114;

    pub const INPUT_PROP_DIRECT: libc::c_int = 0x01;

    const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> libc::Ioctl {
        ((dir << 30) | (size << 16) | (ty << 8) | nr) as libc::Ioctl
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct InputEvent {
        pub time: libc::timeval,
        pub type_: u16,
        pub code: u16,
        pub value: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct InputId {
        bustype: u16,
        vendor: u16,
        product: u16,
        version: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct UinputSetup {
        id: InputId,
        name: [u8; 80],
        ff_effects_max: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct InputAbsInfo {
        value: i32,
        minimum: i32,
        maximum: i32,
        fuzz: i32,
        flat: i32,
        resolution: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct UinputAbsSetup {
        code: u16,
        _pad: u16,
        absinfo: InputAbsInfo,
    }

    const UI_DEV_CREATE: libc::Ioctl = ioc(0, 0x55, 1, 0);
    const UI_DEV_DESTROY: libc::Ioctl = ioc(0, 0x55, 2, 0);
    const UI_DEV_SETUP: libc::Ioctl = ioc(1, 0x55, 3, size_of::<UinputSetup>() as u32);
    const UI_ABS_SETUP: libc::Ioctl = ioc(1, 0x55, 4, size_of::<UinputAbsSetup>() as u32);
    const UI_SET_EVBIT: libc::Ioctl = ioc(1, 0x55, 100, size_of::<libc::c_int>() as u32);
    const UI_SET_KEYBIT: libc::Ioctl = ioc(1, 0x55, 101, size_of::<libc::c_int>() as u32);
    const UI_SET_RELBIT: libc::Ioctl = ioc(1, 0x55, 102, size_of::<libc::c_int>() as u32);
    const UI_SET_ABSBIT: libc::Ioctl = ioc(1, 0x55, 103, size_of::<libc::c_int>() as u32);
    const UI_SET_PROPBIT: libc::Ioctl = ioc(1, 0x55, 110, size_of::<libc::c_int>() as u32);

    pub fn eviocgname(len: usize) -> libc::Ioctl {
        ioc(2, 0x45, 0x06, len as u32)
    }

    pub fn eviocgbit(ev: u32, len: usize) -> libc::Ioctl {
        ioc(2, 0x45, 0x20 + ev, len as u32)
    }

    pub fn is_wayland_session() -> bool {
        if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
            return true;
        }
        std::env::var("XDG_SESSION_TYPE")
            .map(|s| s.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
    }

    pub fn is_hyprland_session() -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some_and(|v| !v.is_empty())
    }

    fn hyprland_socket_path() -> Option<PathBuf> {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        if sig.is_empty() {
            return None;
        }
        if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(xdg).join("hypr").join(&sig).join(".socket.sock");
            if p.exists() {
                return Some(p);
            }
        }
        let legacy = PathBuf::from("/tmp/hypr").join(&sig).join(".socket.sock");
        if legacy.exists() {
            Some(legacy)
        } else {
            None
        }
    }

    pub fn hyprland_cmd(cmd: &str) -> Option<String> {
        let path = hyprland_socket_path()?;
        let mut stream = UnixStream::connect(path).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(120)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(120)));
        stream.write_all(cmd.as_bytes()).ok()?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).ok()?;
        Some(resp)
    }

    pub fn hyprland_cursor_pos() -> Option<(i32, i32)> {
        let resp = hyprland_cmd("cursorpos")?;
        super::parse_hyprland_cursorpos(&resp)
    }

    pub fn hyprland_move_cursor(x: i32, y: i32) -> bool {
        hyprland_cmd(&format!("dispatch movecursor {x} {y}"))
            .is_some_and(|r| r.trim() == "ok" || !r.trim().is_empty())
    }

    pub fn hyprland_virtual_bounds() -> Option<(i32, i32, i32, i32)> {
        let json = hyprland_cmd("j/monitors")?;
        super::parse_hyprland_monitors_bounds(&json)
    }

    fn make_event(type_: u16, code: u16, value: i32) -> InputEvent {
        InputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_,
            code,
            value,
        }
    }

    fn emit_events(file: &mut File, events: &[InputEvent]) {
        let mut buf = Vec::with_capacity((events.len() + 1) * size_of::<InputEvent>());
        let push = |out: &mut Vec<u8>, ev: &InputEvent| {
            let bytes = unsafe {
                std::slice::from_raw_parts((ev as *const InputEvent) as *const u8, size_of::<InputEvent>())
            };
            out.extend_from_slice(bytes);
        };
        for ev in events {
            push(&mut buf, ev);
        }
        let syn = make_event(EV_SYN, SYN_REPORT, 0);
        push(&mut buf, &syn);
        let _ = file.write_all(&buf);
    }

    fn setup_device_name(dev_name: &str) -> UinputSetup {
        let mut setup: UinputSetup = unsafe { zeroed() };
        setup.id = InputId {
            bustype: 0x03, // BUS_USB
            vendor: 0x1209,
            product: 0x4346,
            version: 1,
        };
        let bytes = dev_name.as_bytes();
        let n = bytes.len().min(79);
        setup.name[..n].copy_from_slice(&bytes[..n]);
        setup
    }

    pub struct UinputMouse {
        file: File,
    }

    impl UinputMouse {
        pub fn open() -> Option<Self> {
            let file = OpenOptions::new().write(true).open("/dev/uinput").ok()?;
            let fd = file.as_raw_fd();
            unsafe {
                if libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_int) < 0 {
                    return None;
                }
                for key in [BTN_LEFT, BTN_RIGHT, BTN_MIDDLE] {
                    if libc::ioctl(fd, UI_SET_KEYBIT, key as libc::c_int) < 0 {
                        return None;
                    }
                }
                if libc::ioctl(fd, UI_SET_EVBIT, EV_REL as libc::c_int) < 0 {
                    return None;
                }
                for rel in [REL_X, REL_Y] {
                    if libc::ioctl(fd, UI_SET_RELBIT, rel as libc::c_int) < 0 {
                        return None;
                    }
                }
                let setup = setup_device_name("ClickForge Virtual Mouse");
                if libc::ioctl(fd, UI_DEV_SETUP, &setup) < 0 {
                    return None;
                }
                if libc::ioctl(fd, UI_DEV_CREATE) < 0 {
                    return None;
                }
            }
            Some(Self { file })
        }

        fn btn_code(btn: MouseButton) -> u16 {
            match btn {
                MouseButton::Left => BTN_LEFT,
                MouseButton::Right => BTN_RIGHT,
                MouseButton::Middle => BTN_MIDDLE,
            }
        }

        pub fn press(&mut self, btn: MouseButton) {
            emit_events(&mut self.file, &[make_event(EV_KEY, Self::btn_code(btn), 1)]);
        }

        pub fn release(&mut self, btn: MouseButton) {
            emit_events(&mut self.file, &[make_event(EV_KEY, Self::btn_code(btn), 0)]);
        }

        pub fn click(&mut self, btn: MouseButton) {
            let code = Self::btn_code(btn);
            emit_events(&mut self.file, &[make_event(EV_KEY, code, 1)]);
            emit_events(&mut self.file, &[make_event(EV_KEY, code, 0)]);
        }

        pub fn move_relative(&mut self, dx: i32, dy: i32) {
            if dx == 0 && dy == 0 {
                return;
            }
            let mut batch = Vec::with_capacity(2);
            if dx != 0 {
                batch.push(make_event(EV_REL, REL_X, dx));
            }
            if dy != 0 {
                batch.push(make_event(EV_REL, REL_Y, dy));
            }
            emit_events(&mut self.file, &batch);
        }
    }

    impl Drop for UinputMouse {
        fn drop(&mut self) {
            unsafe {
                libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY);
            }
        }
    }

    pub struct UinputAbsPointer {
        file: File,
    }

    impl UinputAbsPointer {
        pub fn open() -> Option<Self> {
            let file = OpenOptions::new().write(true).open("/dev/uinput").ok()?;
            let fd = file.as_raw_fd();
            unsafe {
                if libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_int) < 0
                    || libc::ioctl(fd, UI_SET_KEYBIT, BTN_LEFT as libc::c_int) < 0
                    || libc::ioctl(fd, UI_SET_EVBIT, EV_ABS as libc::c_int) < 0
                    || libc::ioctl(fd, UI_SET_ABSBIT, ABS_X as libc::c_int) < 0
                    || libc::ioctl(fd, UI_SET_ABSBIT, ABS_Y as libc::c_int) < 0
                {
                    return None;
                }
                let _ = libc::ioctl(fd, UI_SET_PROPBIT, INPUT_PROP_DIRECT);
                let abs_info = InputAbsInfo {
                    value: 0,
                    minimum: 0,
                    maximum: 65535,
                    fuzz: 0,
                    flat: 0,
                    resolution: 0,
                };
                for axis in [ABS_X, ABS_Y] {
                    let abs_setup = UinputAbsSetup {
                        code: axis,
                        _pad: 0,
                        absinfo: abs_info,
                    };
                    if libc::ioctl(fd, UI_ABS_SETUP, &abs_setup) < 0 {
                        return None;
                    }
                }
                let setup = setup_device_name("ClickForge Virtual Pointer");
                if libc::ioctl(fd, UI_DEV_SETUP, &setup) < 0 {
                    return None;
                }
                if libc::ioctl(fd, UI_DEV_CREATE) < 0 {
                    return None;
                }
            }
            Some(Self { file })
        }

        pub fn move_normalized(&mut self, abs_x: i32, abs_y: i32) {
            emit_events(
                &mut self.file,
                &[
                    make_event(EV_ABS, ABS_X, abs_x.clamp(0, 65535)),
                    make_event(EV_ABS, ABS_Y, abs_y.clamp(0, 65535)),
                ],
            );
        }
    }

    impl Drop for UinputAbsPointer {
        fn drop(&mut self) {
            unsafe {
                libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY);
            }
        }
    }

    pub fn has_bit(bits: &[u8], code: usize) -> bool {
        let byte = code / 8;
        let bit = code % 8;
        bits.get(byte).is_some_and(|b| (b & (1 << bit)) != 0)
    }

    pub fn open_evdev_hotkey_devices() -> Vec<RawFd> {
        let Ok(entries) = fs::read_dir("/dev/input") else {
            return Vec::new();
        };
        let mut fds = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with("event") {
                continue;
            }
            let Ok(cpath) = CString::new(entry.path().as_os_str().as_bytes()) else {
                continue;
            };
            let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK) };
            if fd < 0 {
                continue;
            }

            let mut dev_name = [0u8; 128];
            let name_len = unsafe {
                libc::ioctl(fd, eviocgname(dev_name.len()), dev_name.as_mut_ptr())
            };
            if name_len > 0 {
                let txt = String::from_utf8_lossy(&dev_name[..(name_len as usize).min(128)]);
                if txt.contains("ClickForge Virtual") {
                    unsafe { libc::close(fd) };
                    continue;
                }
            }

            let mut ev_bits = [0u8; 8];
            if unsafe { libc::ioctl(fd, eviocgbit(0, ev_bits.len()), ev_bits.as_mut_ptr()) } < 0
                || !has_bit(&ev_bits, EV_KEY as usize)
            {
                unsafe { libc::close(fd) };
                continue;
            }

            let mut key_bits = [0u8; 96]; // covers codes 0..767 (includes BTN_SIDE/EXTRA & F1..F24)
            if unsafe {
                libc::ioctl(
                    fd,
                    eviocgbit(EV_KEY as u32, key_bits.len()),
                    key_bits.as_mut_ptr(),
                )
            } < 0
            {
                unsafe { libc::close(fd) };
                continue;
            }

            // Keep real keyboards (KEY_ESC=1 or KEY_A=30) or mice with side buttons (BTN_SIDE/EXTRA)
            let is_relevant = has_bit(&key_bits, 1)
                || has_bit(&key_bits, 30)
                || has_bit(&key_bits, BTN_SIDE as usize)
                || has_bit(&key_bits, BTN_EXTRA as usize);

            if is_relevant {
                fds.push(fd);
            } else {
                unsafe { libc::close(fd) };
            }
        }
        fds
    }

    pub fn can_write_uinput() -> bool {
        OpenOptions::new().write(true).open("/dev/uinput").is_ok()
    }

    pub fn can_read_evdev() -> bool {
        let fds = open_evdev_hotkey_devices();
        let ok = !fds.is_empty();
        for fd in fds {
            unsafe { libc::close(fd) };
        }
        ok
    }

    pub fn wayland_permission_warning() -> Option<String> {
        if !is_wayland_session() {
            return None;
        }
        let uinput_ok = can_write_uinput();
        let evdev_ok = can_read_evdev();
        if uinput_ok && evdev_ok {
            return None;
        }
        let mut missing = Vec::new();
        if !uinput_ok {
            missing.push("/dev/uinput (virtual mouse click injection)");
        }
        if !evdev_ok {
            missing.push("/dev/input/event* (global keyboard & Mouse4/5 hotkeys)");
        }
        Some(format!(
            "Wayland native clicking & hotkeys need permission for {}.\n\
             Run: sudo usermod -aG input $USER && echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\"' | sudo tee /etc/udev/rules.d/99-clickforge.rules\n\
             Then log out and back in (or see README for Hyprland/Wayland setup).",
            missing.join(" and ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evdev_codes_map_cleanly() {
        assert_eq!(evdev_code_to_name(1).as_deref(), Some("esc"));
        assert_eq!(evdev_code_to_name(64).as_deref(), Some("f6"));
        assert_eq!(evdev_code_to_name(88).as_deref(), Some("f12"));
        assert_eq!(evdev_code_to_name(183).as_deref(), Some("f13"));
        assert_eq!(evdev_code_to_name(0x113).as_deref(), Some("mouse:x1"));
        assert_eq!(evdev_code_to_name(0x114).as_deref(), Some("mouse:x2"));
        assert_eq!(evdev_code_to_name(82).as_deref(), Some("vk96"));
        assert_eq!(evdev_code_to_name(30).as_deref(), Some("a"));
    }

    #[test]
    fn hyprland_cursorpos_and_monitors_parse() {
        assert_eq!(parse_hyprland_cursorpos("1280, 720\n"), Some((1280, 720)));
        assert_eq!(parse_hyprland_cursorpos("-420, 15"), Some((-420, 15)));
        assert_eq!(parse_hyprland_cursorpos("bad"), None);

        let monitors = r#"[
          {"id":0,"x":0,"y":0,"width":2560,"height":1440,"scale":1.0,"transform":0,"disabled":false},
          {"id":1,"x":2560,"y":0,"width":1920,"height":1080,"scale":1.5,"transform":0,"disabled":false},
          {"id":2,"x":5000,"y":0,"width":800,"height":600,"scale":1.0,"transform":0,"disabled":true}
        ]"#;
        // Monitor 1 logical width = 1920 / 1.5 = 1280 -> right = 3840
        assert_eq!(
            parse_hyprland_monitors_bounds(monitors),
            Some((0, 0, 3840, 1440))
        );
    }
}
