use std::sync::Mutex;

pub const INJECTED_MARKER: usize = 0x434C_4B46; // "CLKF"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "right" => Self::Right,
            "middle" => Self::Middle,
            _ => Self::Left,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
        }
    }
}

pub trait InputBackend: Send + Sync {
    fn press(&self, button: MouseButton);
    fn release(&self, button: MouseButton);
    fn click(&self, button: MouseButton) {
        self.press(button);
        self.release(button);
    }
    fn move_to(&self, x: i32, y: i32);
    fn move_relative(&self, _dx: i32, _dy: i32) {}
    fn position(&self) -> Option<(i32, i32)>;
}

// ── Dummy Backend (unit tests & CLI smoke-test) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DummyEvent {
    Move(i32, i32),
    MoveRel(i32, i32),
    Press(MouseButton),
    Release(MouseButton),
    Click(MouseButton),
}

pub struct DummyBackend {
    events: Mutex<Vec<DummyEvent>>,
    cursor: Mutex<Option<(i32, i32)>>,
}

impl DummyBackend {
    pub fn new(initial_pos: (i32, i32)) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            cursor: Mutex::new(Some(initial_pos)),
        }
    }

    #[cfg(test)]
    pub fn with_unknown_pos() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            cursor: Mutex::new(None),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn events(&self) -> Vec<DummyEvent> {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    #[allow(dead_code)]
    pub fn set_cursor(&self, pos: (i32, i32)) {
        *self.cursor.lock().unwrap_or_else(|e| e.into_inner()) = Some(pos);
    }
}

impl InputBackend for DummyBackend {
    fn press(&self, button: MouseButton) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(DummyEvent::Press(button));
    }

    fn release(&self, button: MouseButton) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(DummyEvent::Release(button));
    }

    fn click(&self, button: MouseButton) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(DummyEvent::Click(button));
    }

    fn move_to(&self, x: i32, y: i32) {
        *self.cursor.lock().unwrap_or_else(|e| e.into_inner()) = Some((x, y));
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(DummyEvent::Move(x, y));
    }

    fn move_relative(&self, dx: i32, dy: i32) {
        let mut cur = self.cursor.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cx, cy)) = *cur {
            *cur = Some((cx + dx, cy + dy));
        }
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(DummyEvent::MoveRel(dx, dy));
    }

    fn position(&self) -> Option<(i32, i32)> {
        *self.cursor.lock().unwrap_or_else(|e| e.into_inner())
    }
}

// ── Windows Native Win32 SendInput Backend ──────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

    fn mouse_input(flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: INJECTED_MARKER,
                },
            },
        }
    }

    fn button_flags(button: MouseButton) -> (u32, u32) {
        match button {
            MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        }
    }

    pub struct NativeBackend;

    impl NativeBackend {
        pub fn new() -> Self {
            Self
        }
    }

    impl InputBackend for NativeBackend {
        fn press(&self, button: MouseButton) {
            let (down, _) = button_flags(button);
            let input = [mouse_input(down)];
            unsafe {
                SendInput(1, input.as_ptr(), size_of::<INPUT>() as i32);
            }
        }

        fn release(&self, button: MouseButton) {
            let (_, up) = button_flags(button);
            let input = [mouse_input(up)];
            unsafe {
                SendInput(1, input.as_ptr(), size_of::<INPUT>() as i32);
            }
        }

        fn click(&self, button: MouseButton) {
            let (down, up) = button_flags(button);
            let inputs = [mouse_input(down), mouse_input(up)];
            unsafe {
                SendInput(2, inputs.as_ptr(), size_of::<INPUT>() as i32);
            }
        }

        fn move_to(&self, x: i32, y: i32) {
            unsafe {
                SetCursorPos(x, y);
            }
        }

        fn position(&self) -> Option<(i32, i32)> {
            let mut pt = POINT { x: 0, y: 0 };
            let ok = unsafe { GetCursorPos(&mut pt) };
            if ok != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }
    }
}

// ── macOS Native CoreGraphics Backend ───────────────────────────────────────

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
    const K_CG_HID_EVENT_TAP: u32 = 0;

    const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
    const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
    const K_CG_EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
    const K_CG_EVENT_RIGHT_MOUSE_UP: u32 = 4;
    const K_CG_EVENT_OTHER_MOUSE_DOWN: u32 = 25;
    const K_CG_EVENT_OTHER_MOUSE_UP: u32 = 26;

    const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;
    const K_CG_MOUSE_BUTTON_RIGHT: u32 = 1;
    const K_CG_MOUSE_BUTTON_CENTER: u32 = 2;

    const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceCreate(state_id: i32) -> *mut c_void;
        fn CGEventCreate(source: *mut c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
        fn CGEventCreateMouseEvent(
            source: *mut c_void,
            mouse_type: u32,
            mouse_cursor_position: CGPoint,
            mouse_button: u32,
        ) -> *mut c_void;
        fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
        fn CGEventPost(tap: u32, event: *mut c_void);
        fn CGWarpMouseCursorPosition(new_cursor_position: CGPoint) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *mut c_void);
    }

    pub struct NativeBackend;

    impl NativeBackend {
        pub fn new() -> Self {
            Self
        }

        fn cursor_point() -> Option<CGPoint> {
            unsafe {
                let ev = CGEventCreate(std::ptr::null_mut());
                if ev.is_null() {
                    return None;
                }
                let pt = CGEventGetLocation(ev);
                CFRelease(ev);
                Some(pt)
            }
        }

        fn post_mouse(ev_type: u32, cg_btn: u32, pt: CGPoint) {
            unsafe {
                let src = CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE);
                let ev = CGEventCreateMouseEvent(src, ev_type, pt, cg_btn);
                if !ev.is_null() {
                    CGEventSetIntegerValueField(
                        ev,
                        K_CG_EVENT_SOURCE_USER_DATA,
                        INJECTED_MARKER as i64,
                    );
                    CGEventPost(K_CG_HID_EVENT_TAP, ev);
                    CFRelease(ev);
                }
                if !src.is_null() {
                    CFRelease(src);
                }
            }
        }

        fn event_types(button: MouseButton) -> (u32, u32, u32) {
            match button {
                MouseButton::Left => (
                    K_CG_EVENT_LEFT_MOUSE_DOWN,
                    K_CG_EVENT_LEFT_MOUSE_UP,
                    K_CG_MOUSE_BUTTON_LEFT,
                ),
                MouseButton::Right => (
                    K_CG_EVENT_RIGHT_MOUSE_DOWN,
                    K_CG_EVENT_RIGHT_MOUSE_UP,
                    K_CG_MOUSE_BUTTON_RIGHT,
                ),
                MouseButton::Middle => (
                    K_CG_EVENT_OTHER_MOUSE_DOWN,
                    K_CG_EVENT_OTHER_MOUSE_UP,
                    K_CG_MOUSE_BUTTON_CENTER,
                ),
            }
        }
    }

    impl InputBackend for NativeBackend {
        fn press(&self, button: MouseButton) {
            let pt = Self::cursor_point().unwrap_or(CGPoint { x: 0.0, y: 0.0 });
            let (down, _, btn) = Self::event_types(button);
            Self::post_mouse(down, btn, pt);
        }

        fn release(&self, button: MouseButton) {
            let pt = Self::cursor_point().unwrap_or(CGPoint { x: 0.0, y: 0.0 });
            let (_, up, btn) = Self::event_types(button);
            Self::post_mouse(up, btn, pt);
        }

        fn click(&self, button: MouseButton) {
            let pt = Self::cursor_point().unwrap_or(CGPoint { x: 0.0, y: 0.0 });
            let (down, up, btn) = Self::event_types(button);
            Self::post_mouse(down, btn, pt);
            Self::post_mouse(up, btn, pt);
        }

        fn move_to(&self, x: i32, y: i32) {
            unsafe {
                CGWarpMouseCursorPosition(CGPoint {
                    x: x as f64,
                    y: y as f64,
                });
            }
        }

        fn position(&self) -> Option<(i32, i32)> {
            Self::cursor_point().map(|p| (p.x.round() as i32, p.y.round() as i32))
        }
    }
}

// ── Linux Hybrid Wayland (Hyprland / uinput) + X11 (XTest) Backend ──────────

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use crate::core::system::virtual_screen_bounds;
    use crate::core::wayland::linux_rt::{
        hyprland_cursor_pos, hyprland_move_cursor, is_hyprland_session, is_wayland_session,
        UinputAbsPointer, UinputMouse,
    };
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        query_pointer, BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT, MOTION_NOTIFY_EVENT,
    };
    use x11rb::protocol::xtest::fake_input;
    use x11rb::rust_connection::RustConnection;

    pub struct NativeBackend {
        is_wayland: bool,
        is_hyprland: bool,
        uinput_mouse: Mutex<Option<UinputMouse>>,
        uinput_abs: Mutex<Option<UinputAbsPointer>>,
        x11: Mutex<Option<(RustConnection, u32)>>,
    }

    impl NativeBackend {
        pub fn new() -> Self {
            let is_wayland = is_wayland_session();
            let is_hyprland = is_hyprland_session();
            let uinput_mouse = if is_wayland {
                UinputMouse::open()
            } else {
                None
            };
            let uinput_abs = if is_wayland && !is_hyprland && uinput_mouse.is_some() {
                UinputAbsPointer::open()
            } else {
                None
            };
            let x11 = x11rb::connect(None).ok().map(|(c, screen_num)| {
                let root = c.setup().roots[screen_num].root;
                (c, root)
            });
            Self {
                is_wayland,
                is_hyprland,
                uinput_mouse: Mutex::new(uinput_mouse),
                uinput_abs: Mutex::new(uinput_abs),
                x11: Mutex::new(x11),
            }
        }

        fn button_detail(button: MouseButton) -> u8 {
            match button {
                MouseButton::Left => 1,
                MouseButton::Middle => 2,
                MouseButton::Right => 3,
            }
        }
    }

    impl InputBackend for NativeBackend {
        fn press(&self, button: MouseButton) {
            {
                let mut ui = self.uinput_mouse.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(m) = ui.as_mut() {
                    m.press(button);
                    return;
                }
            }
            let guard = self.x11.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((conn, _)) = guard.as_ref() {
                let _ = fake_input(
                    conn,
                    BUTTON_PRESS_EVENT,
                    Self::button_detail(button),
                    x11rb::CURRENT_TIME,
                    x11rb::NONE,
                    0,
                    0,
                    0,
                );
                let _ = conn.flush();
            }
        }

        fn release(&self, button: MouseButton) {
            {
                let mut ui = self.uinput_mouse.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(m) = ui.as_mut() {
                    m.release(button);
                    return;
                }
            }
            let guard = self.x11.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((conn, _)) = guard.as_ref() {
                let _ = fake_input(
                    conn,
                    BUTTON_RELEASE_EVENT,
                    Self::button_detail(button),
                    x11rb::CURRENT_TIME,
                    x11rb::NONE,
                    0,
                    0,
                    0,
                );
                let _ = conn.flush();
            }
        }

        fn click(&self, button: MouseButton) {
            {
                let mut ui = self.uinput_mouse.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(m) = ui.as_mut() {
                    m.click(button);
                    return;
                }
            }
            self.press(button);
            self.release(button);
        }

        fn move_to(&self, x: i32, y: i32) {
            if self.is_wayland && self.is_hyprland && hyprland_move_cursor(x, y) {
                return;
            }
            if self.is_wayland {
                let mut abs = self.uinput_abs.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ptr) = abs.as_mut() {
                    let (left, top, width, height) =
                        virtual_screen_bounds().unwrap_or((0, 0, 1920, 1080));
                    let nx = (((x - left) as f64 / width.max(1) as f64) * 65535.0).round() as i32;
                    let ny = (((y - top) as f64 / height.max(1) as f64) * 65535.0).round() as i32;
                    ptr.move_normalized(nx, ny);
                    return;
                }
            }
            let guard = self.x11.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((conn, root)) = guard.as_ref() {
                let _ = fake_input(
                    conn,
                    MOTION_NOTIFY_EVENT,
                    0,
                    x11rb::CURRENT_TIME,
                    *root,
                    x.clamp(-32768, 32767) as i16,
                    y.clamp(-32768, 32767) as i16,
                    0,
                );
                let _ = conn.flush();
            }
        }

        fn move_relative(&self, dx: i32, dy: i32) {
            if self.is_wayland {
                let mut ui = self.uinput_mouse.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(m) = ui.as_mut() {
                    m.move_relative(dx, dy);
                }
            }
        }

        fn position(&self) -> Option<(i32, i32)> {
            if self.is_wayland && self.is_hyprland {
                if let Some(pos) = hyprland_cursor_pos() {
                    return Some(pos);
                }
            }
            if self.is_wayland {
                return None;
            }
            let guard = self.x11.lock().unwrap_or_else(|e| e.into_inner());
            let (conn, root) = guard.as_ref()?;
            let reply = query_pointer(conn, *root).ok()?.reply().ok()?;
            Some((reply.root_x as i32, reply.root_y as i32))
        }
    }
}

pub fn create_native_backend() -> Box<dyn InputBackend> {
    Box::new(platform::NativeBackend::new())
}
