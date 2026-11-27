use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const PANIC_KEY: &str = "esc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerEvent {
    Down,
    Up,
    Panic,
    Recorded(String),
    RecordCancelled,
}

pub type TriggerCallback = Arc<dyn Fn(TriggerEvent) + Send + Sync>;

pub fn vk_to_name(vk: u32) -> Option<String> {
    let s: &str = match vk {
        8 => "backspace",
        9 => "tab",
        13 => "enter",
        17 => "ctrl",
        18 => "alt",
        19 => "pause",
        20 => "caps_lock",
        27 => "esc",
        32 => "space",
        33 => "page_up",
        34 => "page_down",
        35 => "end",
        36 => "home",
        37 => "left",
        38 => "up",
        39 => "right",
        40 => "down",
        44 => "print_screen",
        45 => "insert",
        46 => "delete",
        91 => "cmd",
        92 => "cmd_r",
        93 => "menu",
        112..=135 => return Some(format!("f{}", vk - 111)),
        144 => "num_lock",
        145 => "scroll_lock",
        160 => "shift",
        161 => "shift_r",
        162 => "ctrl_l",
        163 => "ctrl_r",
        164 => "alt_l",
        165 => "alt_r",
        173 => "media_volume_mute",
        174 => "media_volume_down",
        175 => "media_volume_up",
        176 => "media_next",
        177 => "media_previous",
        178 => "media_stop",
        179 => "media_play_pause",
        0x30..=0x39 => return Some(((vk as u8) as char).to_string()),
        0x41..=0x5A => return Some(((vk as u8 as char).to_ascii_lowercase()).to_string()),
        _ => return None,
    };
    Some(s.to_string())
}

pub fn normalize_binding(binding: &str, default: &str) -> String {
    let cleaned = binding.trim().to_ascii_lowercase();
    if cleaned.is_empty() {
        return default.to_string();
    }
    if let Some(digits) = cleaned.strip_prefix("vk") {
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(vk) = digits.parse::<u32>() {
                if let Some(name) = vk_to_name(vk) {
                    return name;
                }
            }
        }
    }
    cleaned
}

pub fn display_name(binding: &str) -> String {
    let norm = normalize_binding(binding, "f6");
    match norm.as_str() {
        "mouse:x1" => return "Mouse4".to_string(),
        "mouse:x2" => return "Mouse5".to_string(),
        "vk96" => return "Num 0".to_string(),
        "vk97" => return "Num 1".to_string(),
        "vk98" => return "Num 2".to_string(),
        "vk99" => return "Num 3".to_string(),
        "vk100" => return "Num 4".to_string(),
        "vk101" => return "Num 5".to_string(),
        "vk102" => return "Num 6".to_string(),
        "vk103" => return "Num 7".to_string(),
        "vk104" => return "Num 8".to_string(),
        "vk105" => return "Num 9".to_string(),
        "vk106" => return "Num *".to_string(),
        "vk107" => return "Num +".to_string(),
        "vk109" => return "Num -".to_string(),
        "vk110" => return "Num .".to_string(),
        "vk111" => return "Num /".to_string(),
        _ => {}
    }

    if let Some(rest) = norm.strip_prefix('f') {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return format!("F{rest}");
        }
    }
    if norm.chars().count() == 1 {
        return norm.to_uppercase();
    }

    let words: Vec<String> = norm
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect();
    if words.is_empty() {
        "F6".to_string()
    } else {
        words.join(" ")
    }
}

pub fn conflicts_with_numeric_entry(binding: &str) -> bool {
    let norm = normalize_binding(binding, "f6");
    if matches!(
        norm.as_str(),
        "0" | "1"
            | "2"
            | "3"
            | "4"
            | "5"
            | "6"
            | "7"
            | "8"
            | "9"
            | "."
            | "-"
            | "backspace"
            | "delete"
            | "left"
            | "right"
            | "home"
            | "end"
    ) {
        return true;
    }
    if let Some(digits) = norm.strip_prefix("vk") {
        if let Ok(code) = digits.parse::<u32>() {
            return (96..=111).contains(&code);
        }
    }
    false
}

struct SharedState {
    binding: String,
    recording: bool,
    suspended: bool,
    held: bool,
}

pub struct TriggerListener {
    shared: Arc<Mutex<SharedState>>,
    #[cfg_attr(not(test), allow(dead_code))]
    callback: TriggerCallback,
    backend: platform::HookBackend,
}

impl TriggerListener {
    pub fn new(binding: &str, callback: TriggerCallback) -> Self {
        let shared = Arc::new(Mutex::new(SharedState {
            binding: normalize_binding(binding, "f6"),
            recording: false,
            suspended: false,
            held: false,
        }));
        Self {
            shared: Arc::clone(&shared),
            callback: Arc::clone(&callback),
            backend: platform::HookBackend::new(shared, callback),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn binding(&self) -> String {
        self.shared
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .binding
            .clone()
    }

    #[allow(dead_code)]
    pub fn set_binding(&self, binding: &str) -> String {
        let normalized = normalize_binding(binding, "f6");
        {
            let mut st = self.shared.lock().unwrap_or_else(|e| e.into_inner());
            st.binding = normalized.clone();
            st.held = false;
        }
        self.backend.notify_config_changed();
        normalized
    }

    pub fn set_recording(&self, active: bool) {
        {
            let mut st = self.shared.lock().unwrap_or_else(|e| e.into_inner());
            st.recording = active;
            st.held = false;
        }
        self.backend.notify_config_changed();
    }

    pub fn set_suspended(&self, suspended: bool) {
        let mut st = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        st.suspended = suspended;
        if suspended {
            st.held = false;
        }
    }

    pub fn reset_trigger_state(&self) {
        let mut st = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        st.held = false;
    }

    pub fn start(&self) {
        self.backend.start();
    }

    pub fn stop(&self) {
        self.backend.stop();
    }

    #[cfg(test)]
    pub fn process_key(&self, name: &str, pressed: bool) {
        dispatch_input(&self.shared, &self.callback, name, pressed);
    }
}

impl Drop for TriggerListener {
    fn drop(&mut self) {
        self.stop();
    }
}

fn dispatch_input(
    shared: &Arc<Mutex<SharedState>>,
    callback: &TriggerCallback,
    name: &str,
    pressed: bool,
) {
    let norm = normalize_binding(name, name);
    if norm.is_empty() {
        return;
    }

    let event: Option<TriggerEvent> = {
        let mut st = shared.lock().unwrap_or_else(|e| e.into_inner());
        if st.recording {
            if !pressed {
                return;
            }
            st.recording = false;
            st.held = false;
            if norm == PANIC_KEY {
                Some(TriggerEvent::RecordCancelled)
            } else {
                st.binding = norm.clone();
                Some(TriggerEvent::Recorded(norm))
            }
        } else if pressed && norm == PANIC_KEY {
            st.held = false;
            Some(TriggerEvent::Panic)
        } else if st.suspended || norm != st.binding {
            None
        } else if pressed {
            if st.held {
                None
            } else {
                st.held = true;
                Some(TriggerEvent::Down)
            }
        } else if st.held {
            st.held = false;
            Some(TriggerEvent::Up)
        } else {
            None
        }
    };

    if let Some(ev) = event {
        callback(ev);
    }
}

// ── Windows Hook Backend (WH_KEYBOARD_LL + on-demand WH_MOUSE_LL) ────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::OnceLock;
    use std::thread::{self, JoinHandle};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN,
        WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    use crate::core::input_backend::INJECTED_MARKER;

    const WM_SYNC_HOOKS: u32 = WM_APP + 1;
    const LLKHF_INJECTED: u32 = 0x10;
    const LLMHF_INJECTED: u32 = 0x01;

    struct ActiveHookCtx {
        shared: Arc<Mutex<SharedState>>,
        callback: TriggerCallback,
    }

    static ACTIVE_CTX: OnceLock<Mutex<Option<Arc<ActiveHookCtx>>>> = OnceLock::new();

    fn ctx_slot() -> &'static Mutex<Option<Arc<ActiveHookCtx>>> {
        ACTIVE_CTX.get_or_init(|| Mutex::new(None))
    }

    unsafe extern "system" fn kb_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && lparam != 0 {
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
            if (kb.flags & LLKHF_INJECTED) == 0 && kb.dwExtraInfo != INJECTED_MARKER {
                let msg = wparam as u32;
                let pressed = match msg {
                    WM_KEYDOWN | WM_SYSKEYDOWN => Some(true),
                    WM_KEYUP | WM_SYSKEYUP => Some(false),
                    _ => None,
                };
                if let Some(is_down) = pressed {
                    let key_name = vk_to_name(kb.vkCode).unwrap_or_else(|| format!("vk{}", kb.vkCode));
                    let ctx = ctx_slot().lock().unwrap_or_else(|e| e.into_inner()).clone();
                    if let Some(c) = ctx {
                        dispatch_input(&c.shared, &c.callback, &key_name, is_down);
                    }
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && lparam != 0 {
            let ms = &*(lparam as *const MSLLHOOKSTRUCT);
            if (ms.flags & LLMHF_INJECTED) == 0 && ms.dwExtraInfo != INJECTED_MARKER {
                let msg = wparam as u32;
                let pressed = match msg {
                    WM_XBUTTONDOWN => Some(true),
                    WM_XBUTTONUP => Some(false),
                    _ => None,
                };
                if let Some(is_down) = pressed {
                    let xbtn = (ms.mouseData >> 16) & 0xFFFF;
                    let name = match xbtn {
                        1 => Some("mouse:x1"),
                        2 => Some("mouse:x2"),
                        _ => None,
                    };
                    if let Some(btn_name) = name {
                        let ctx = ctx_slot().lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if let Some(c) = ctx {
                            dispatch_input(&c.shared, &c.callback, btn_name, is_down);
                        }
                    }
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    pub struct HookBackend {
        shared: Arc<Mutex<SharedState>>,
        callback: TriggerCallback,
        thread_id: Arc<AtomicU32>,
        running: Arc<AtomicBool>,
        handle: Mutex<Option<JoinHandle<()>>>,
    }

    impl HookBackend {
        pub fn new(shared: Arc<Mutex<SharedState>>, callback: TriggerCallback) -> Self {
            Self {
                shared,
                callback,
                thread_id: Arc::new(AtomicU32::new(0)),
                running: Arc::new(AtomicBool::new(false)),
                handle: Mutex::new(None),
            }
        }

        pub fn start(&self) {
            let mut slot = self.handle.lock().unwrap_or_else(|e| e.into_inner());
            if slot.is_some() {
                return;
            }
            self.running.store(true, Ordering::Release);
            *ctx_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(ActiveHookCtx {
                shared: Arc::clone(&self.shared),
                callback: Arc::clone(&self.callback),
            }));

            let shared = Arc::clone(&self.shared);
            let tid_out = Arc::clone(&self.thread_id);
            let running = Arc::clone(&self.running);

            let handle = thread::Builder::new()
                .name("ClickForge-Hotkeys".to_string())
                .spawn(move || unsafe {
                    let tid = GetCurrentThreadId();
                    tid_out.store(tid, Ordering::Release);

                    let kb_hook: HHOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kb_proc), std::ptr::null_mut(), 0);
                    let mut mouse_hook: HHOOK = std::ptr::null_mut();

                    let sync_mouse = |mh: &mut HHOOK| {
                        let need = {
                            let st = shared.lock().unwrap_or_else(|e| e.into_inner());
                            st.recording || st.binding.starts_with("mouse:")
                        };
                        if need && mh.is_null() {
                            *mh = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), std::ptr::null_mut(), 0);
                        } else if !need && !mh.is_null() {
                            UnhookWindowsHookEx(*mh);
                            *mh = std::ptr::null_mut();
                        }
                    };
                    sync_mouse(&mut mouse_hook);

                    let mut msg: MSG = std::mem::zeroed();
                    while running.load(Ordering::Acquire) {
                        let res = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                        if res <= 0 {
                            break;
                        }
                        if msg.message == WM_SYNC_HOOKS {
                            sync_mouse(&mut mouse_hook);
                            continue;
                        }
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                        sync_mouse(&mut mouse_hook);
                    }

                    if !mouse_hook.is_null() {
                        UnhookWindowsHookEx(mouse_hook);
                    }
                    if !kb_hook.is_null() {
                        UnhookWindowsHookEx(kb_hook);
                    }
                    tid_out.store(0, Ordering::Release);
                })
                .ok();

            *slot = handle;
        }

        pub fn notify_config_changed(&self) {
            let tid = self.thread_id.load(Ordering::Acquire);
            if tid != 0 {
                unsafe {
                    PostThreadMessageW(tid, WM_SYNC_HOOKS, 0, 0);
                }
            }
        }

        pub fn stop(&self) {
            self.running.store(false, Ordering::Release);
            let tid = self.thread_id.load(Ordering::Acquire);
            if tid != 0 {
                unsafe {
                    PostThreadMessageW(tid, WM_QUIT, 0, 0);
                }
            }
            if let Some(h) = self.handle.lock().unwrap_or_else(|e| e.into_inner()).take() {
                let _ = h.join();
            }
            *ctx_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
}

// ── macOS CGEventTap Backend ─────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::sync::atomic::AtomicUsize;
    use std::thread::{self, JoinHandle};

    const K_CG_SESSION_EVENT_TAP: u32 = 1;
    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

    const K_CG_EVENT_KEY_DOWN: u32 = 10;
    const K_CG_EVENT_KEY_UP: u32 = 11;
    const K_CG_EVENT_OTHER_MOUSE_DOWN: u32 = 25;
    const K_CG_EVENT_OTHER_MOUSE_UP: u32 = 26;
    const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;
    const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFFFFFF;

    const K_CG_MOUSE_EVENT_BUTTON_NUMBER: u32 = 3;
    const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
    const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
            user_info: *mut c_void,
        ) -> *mut c_void;
        fn CGEventTapEnable(tap: *mut c_void, enable: bool);
        fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
        fn CGEventKeyboardGetUnicodeString(
            event: *mut c_void,
            max_string_length: usize,
            actual_string_length: *mut usize,
            unicode_string: *mut u16,
        );
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFRunLoopCommonModes: *const c_void;
        fn CFMachPortCreateRunLoopSource(
            allocator: *mut c_void,
            port: *mut c_void,
            order: isize,
        ) -> *mut c_void;
        fn CFRunLoopGetCurrent() -> *mut c_void;
        fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
        fn CFRunLoopRun();
        fn CFRunLoopStop(rl: *mut c_void);
        fn CFRelease(cf: *mut c_void);
    }

    fn mac_keycode_to_name(kc: i64, ev: *mut c_void) -> Option<String> {
        let special = match kc {
            0x7A => Some("f1"),
            0x78 => Some("f2"),
            0x63 => Some("f3"),
            0x76 => Some("f4"),
            0x60 => Some("f5"),
            0x61 => Some("f6"),
            0x62 => Some("f7"),
            0x64 => Some("f8"),
            0x65 => Some("f9"),
            0x6D => Some("f10"),
            0x67 => Some("f11"),
            0x6F => Some("f12"),
            0x69 => Some("f13"),
            0x6B => Some("f14"),
            0x71 => Some("f15"),
            0x6A => Some("f16"),
            0x40 => Some("f17"),
            0x4F => Some("f18"),
            0x50 => Some("f19"),
            0x5A => Some("f20"),
            0x35 => Some("esc"),
            0x31 => Some("space"),
            0x24 => Some("enter"),
            0x30 => Some("tab"),
            0x33 => Some("backspace"),
            0x75 => Some("delete"),
            0x7B => Some("left"),
            0x7C => Some("right"),
            0x7D => Some("down"),
            0x7E => Some("up"),
            0x73 => Some("home"),
            0x77 => Some("end"),
            0x74 => Some("page_up"),
            0x79 => Some("page_down"),
            _ => None,
        };
        if let Some(s) = special {
            return Some(s.to_string());
        }
        let mut len: usize = 0;
        let mut buf = [0u16; 4];
        unsafe {
            CGEventKeyboardGetUnicodeString(ev, 4, &mut len, buf.as_mut_ptr());
        }
        if len > 0 {
            if let Some(ch) = std::char::from_u32(buf[0] as u32) {
                if !ch.is_control() {
                    return Some(ch.to_lowercase().to_string());
                }
            }
        }
        None
    }

    struct TapUserData {
        shared: Arc<Mutex<SharedState>>,
        callback: TriggerCallback,
        tap_port: usize,
    }

    extern "C" fn tap_cb(
        _proxy: *mut c_void,
        ev_type: u32,
        event: *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void {
        if user_info.is_null() {
            return event;
        }
        let ctx = unsafe { &*(user_info as *const TapUserData) };
        if ev_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
            || ev_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
        {
            if ctx.tap_port != 0 {
                unsafe { CGEventTapEnable(ctx.tap_port as *mut c_void, true) };
            }
            return event;
        }
        let marker = unsafe { CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA) };
        if marker == crate::core::input_backend::INJECTED_MARKER as i64 {
            return event;
        }

        match ev_type {
            K_CG_EVENT_KEY_DOWN | K_CG_EVENT_KEY_UP => {
                let kc = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) };
                if let Some(name) = mac_keycode_to_name(kc, event) {
                    dispatch_input(
                        &ctx.shared,
                        &ctx.callback,
                        &name,
                        ev_type == K_CG_EVENT_KEY_DOWN,
                    );
                }
            }
            K_CG_EVENT_OTHER_MOUSE_DOWN | K_CG_EVENT_OTHER_MOUSE_UP => {
                let btn = unsafe { CGEventGetIntegerValueField(event, K_CG_MOUSE_EVENT_BUTTON_NUMBER) };
                let name = match btn {
                    3 => Some("mouse:x1"),
                    4 => Some("mouse:x2"),
                    _ => None,
                };
                if let Some(n) = name {
                    dispatch_input(
                        &ctx.shared,
                        &ctx.callback,
                        n,
                        ev_type == K_CG_EVENT_OTHER_MOUSE_DOWN,
                    );
                }
            }
            _ => {}
        }
        event
    }

    pub struct HookBackend {
        shared: Arc<Mutex<SharedState>>,
        callback: TriggerCallback,
        run_loop: Arc<AtomicUsize>,
        running: Arc<AtomicBool>,
        handle: Mutex<Option<JoinHandle<()>>>,
    }

    impl HookBackend {
        pub fn new(shared: Arc<Mutex<SharedState>>, callback: TriggerCallback) -> Self {
            Self {
                shared,
                callback,
                run_loop: Arc::new(AtomicUsize::new(0)),
                running: Arc::new(AtomicBool::new(false)),
                handle: Mutex::new(None),
            }
        }

        pub fn start(&self) {
            let mut slot = self.handle.lock().unwrap_or_else(|e| e.into_inner());
            if slot.is_some() {
                return;
            }
            self.running.store(true, Ordering::Release);
            let shared = Arc::clone(&self.shared);
            let cb = Arc::clone(&self.callback);
            let rl_slot = Arc::clone(&self.run_loop);

            let handle = thread::Builder::new()
                .name("ClickForge-Hotkeys".to_string())
                .spawn(move || unsafe {
                    let user_data = Box::new(TapUserData {
                        shared,
                        callback: cb,
                        tap_port: 0,
                    });
                    let user_ptr = Box::into_raw(user_data);
                    let mask = (1u64 << K_CG_EVENT_KEY_DOWN)
                        | (1u64 << K_CG_EVENT_KEY_UP)
                        | (1u64 << K_CG_EVENT_OTHER_MOUSE_DOWN)
                        | (1u64 << K_CG_EVENT_OTHER_MOUSE_UP);
                    let tap = CGEventTapCreate(
                        K_CG_SESSION_EVENT_TAP,
                        K_CG_HEAD_INSERT_EVENT_TAP,
                        K_CG_EVENT_TAP_OPTION_LISTEN_ONLY,
                        mask,
                        tap_cb,
                        user_ptr as *mut c_void,
                    );
                    if !tap.is_null() {
                        (*user_ptr).tap_port = tap as usize;
                        let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap, 0);
                        if !source.is_null() {
                            let rl = CFRunLoopGetCurrent();
                            rl_slot.store(rl as usize, Ordering::Release);
                            CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
                            CGEventTapEnable(tap, true);
                            CFRunLoopRun();
                            rl_slot.store(0, Ordering::Release);
                            CFRelease(source);
                        }
                        CFRelease(tap);
                    }
                    drop(Box::from_raw(user_ptr));
                })
                .ok();
            *slot = handle;
        }

        pub fn notify_config_changed(&self) {}

        pub fn stop(&self) {
            self.running.store(false, Ordering::Release);
            let rl = self.run_loop.load(Ordering::Acquire);
            if rl != 0 {
                unsafe { CFRunLoopStop(rl as *mut c_void) };
            }
            if let Some(h) = self.handle.lock().unwrap_or_else(|e| e.into_inner()).take() {
                let _ = h.join();
            }
        }
    }
}

// ── Linux Wayland (evdev) + X11 Record Extension Backend ─────────────────────

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use crate::core::wayland::{evdev_code_to_name, linux_rt};
    use std::collections::HashMap;
    use std::mem::size_of;
    use std::sync::atomic::AtomicU32;
    use std::thread::{self, JoinHandle};
    use x11rb::connection::Connection;
    use x11rb::protocol::record::{
        self, ConnectionExt as _, CS, ExtRange, Range, Range16, Range8,
    };
    use x11rb::protocol::xproto::ConnectionExt as _;

    fn keysym_to_name(sym: u32) -> Option<String> {
        let special = match sym {
            0xFFBE..=0xFFD5 => return Some(format!("f{}", sym - 0xFFBD)),
            0xFF1B => "esc",
            0x0020 => "space",
            0xFF0D => "enter",
            0xFF09 => "tab",
            0xFF08 => "backspace",
            0xFFFF => "delete",
            0xFF50 => "home",
            0xFF51 => "left",
            0xFF52 => "up",
            0xFF53 => "right",
            0xFF54 => "down",
            0xFF55 => "page_up",
            0xFF56 => "page_down",
            0xFF57 => "end",
            0xFF63 => "insert",
            0x0021..=0x007E => {
                if let Some(c) = std::char::from_u32(sym) {
                    return Some(c.to_ascii_lowercase().to_string());
                }
                return None;
            }
            _ => return None,
        };
        Some(special.to_string())
    }

    fn run_evdev_loop(
        shared: &Arc<Mutex<SharedState>>,
        callback: &TriggerCallback,
        running: &Arc<AtomicBool>,
        fds: Vec<std::os::unix::io::RawFd>,
    ) {
        let mut pollfds: Vec<libc::pollfd> = fds
            .iter()
            .map(|&fd| libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        let mut ev: linux_rt::InputEvent = unsafe { std::mem::zeroed() };
        let ev_size = size_of::<linux_rt::InputEvent>() as libc::size_t;

        while running.load(Ordering::Acquire) {
            let ready = unsafe {
                libc::poll(
                    pollfds.as_mut_ptr(),
                    pollfds.len() as libc::nfds_t,
                    150,
                )
            };
            if ready < 0 {
                let err = std::io::Error::last_os_error().raw_os_error();
                if err == Some(libc::EINTR) {
                    continue;
                }
                break;
            }
            if ready == 0 {
                continue;
            }
            for pfd in &mut pollfds {
                if (pfd.revents & libc::POLLIN) == 0 {
                    continue;
                }
                loop {
                    let n = unsafe {
                        libc::read(
                            pfd.fd,
                            (&mut ev as *mut linux_rt::InputEvent) as *mut libc::c_void,
                            ev_size,
                        )
                    };
                    if n != ev_size as isize {
                        break;
                    }
                    if ev.type_ == linux_rt::EV_KEY && (ev.value == 0 || ev.value == 1) {
                        if let Some(name) = evdev_code_to_name(ev.code) {
                            dispatch_input(shared, callback, &name, ev.value == 1);
                        }
                    }
                }
            }
        }

        for fd in fds {
            unsafe { libc::close(fd) };
        }
    }

    pub struct HookBackend {
        shared: Arc<Mutex<SharedState>>,
        callback: TriggerCallback,
        context_id: Arc<AtomicU32>,
        running: Arc<AtomicBool>,
        handle: Mutex<Option<JoinHandle<()>>>,
    }

    impl HookBackend {
        pub fn new(shared: Arc<Mutex<SharedState>>, callback: TriggerCallback) -> Self {
            Self {
                shared,
                callback,
                context_id: Arc::new(AtomicU32::new(0)),
                running: Arc::new(AtomicBool::new(false)),
                handle: Mutex::new(None),
            }
        }

        pub fn start(&self) {
            let mut slot = self.handle.lock().unwrap_or_else(|e| e.into_inner());
            if slot.is_some() {
                return;
            }
            self.running.store(true, Ordering::Release);
            let shared = Arc::clone(&self.shared);
            let callback = Arc::clone(&self.callback);
            let ctx_slot = Arc::clone(&self.context_id);
            let running = Arc::clone(&self.running);

            let handle = thread::Builder::new()
                .name("ClickForge-Hotkeys".to_string())
                .spawn(move || {
                    if linux_rt::is_wayland_session() {
                        let evdev_fds = linux_rt::open_evdev_hotkey_devices();
                        if !evdev_fds.is_empty() {
                            run_evdev_loop(&shared, &callback, &running, evdev_fds);
                            return;
                        }
                    }

                    let Ok((data_conn, _)) = x11rb::connect(None) else {
                        return;
                    };
                    let Ok((ctrl_conn, _)) = x11rb::connect(None) else {
                        return;
                    };
                    if data_conn.record_query_version(1, 13).is_err() {
                        return;
                    }

                    let setup = ctrl_conn.setup();
                    let min_kc = setup.min_keycode;
                    let count = setup.max_keycode.saturating_sub(min_kc).saturating_add(1);
                    let mut kc_map: HashMap<u8, String> = HashMap::new();
                    if let Ok(cookie) = ctrl_conn.get_keyboard_mapping(min_kc, count) {
                        if let Ok(reply) = cookie.reply() {
                            let per = reply.keysyms_per_keycode as usize;
                            if per > 0 {
                                for (idx, chunk) in reply.keysyms.chunks(per).enumerate() {
                                    if let Some(name) = keysym_to_name(chunk[0]) {
                                        kc_map.insert(min_kc.saturating_add(idx as u8), name);
                                    }
                                }
                            }
                        }
                    }

                    let Ok(ctx) = data_conn.generate_id() else {
                        return;
                    };
                    let empty_ext = ExtRange {
                        major: Range8 { first: 0, last: 0 },
                        minor: Range16 { first: 0, last: 0 },
                    };
                    let range = Range {
                        core_requests: Range8 { first: 0, last: 0 },
                        core_replies: Range8 { first: 0, last: 0 },
                        ext_requests: empty_ext,
                        ext_replies: empty_ext,
                        delivered_events: Range8 { first: 0, last: 0 },
                        device_events: Range8 { first: 2, last: 5 }, // KeyPress..ButtonRelease
                        errors: Range8 { first: 0, last: 0 },
                        client_started: false,
                        client_died: false,
                    };
                    if data_conn
                        .record_create_context(ctx, 0, &[u32::from(CS::ALL_CLIENTS)], &[range])
                        .is_err()
                    {
                        return;
                    }
                    let _ = data_conn.flush();
                    ctx_slot.store(ctx, Ordering::Release);

                    if let Ok(iter) = data_conn.record_enable_context(ctx) {
                        for item in iter {
                            if !running.load(Ordering::Acquire) {
                                break;
                            }
                            let Ok(reply) = item else {
                                break;
                            };
                            if reply.category != 0 {
                                continue;
                            }
                            for pkt in reply.data.chunks_exact(32) {
                                let ev_type = pkt[0] & 0x7F;
                                let detail = pkt[1];
                                match ev_type {
                                    2 | 3 => {
                                        if let Some(name) = kc_map.get(&detail) {
                                            dispatch_input(&shared, &callback, name, ev_type == 2);
                                        }
                                    }
                                    4 | 5 => {
                                        let mname = match detail {
                                            8 => Some("mouse:x1"),
                                            9 => Some("mouse:x2"),
                                            _ => None,
                                        };
                                        if let Some(n) = mname {
                                            dispatch_input(&shared, &callback, n, ev_type == 4);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    let _ = record::disable_context(&ctrl_conn, ctx);
                    let _ = record::free_context(&ctrl_conn, ctx);
                    let _ = ctrl_conn.flush();
                    ctx_slot.store(0, Ordering::Release);
                })
                .ok();
            *slot = handle;
        }

        pub fn notify_config_changed(&self) {}

        pub fn stop(&self) {
            self.running.store(false, Ordering::Release);
            let ctx = self.context_id.load(Ordering::Acquire);
            if ctx != 0 {
                if let Ok((ctrl, _)) = x11rb::connect(None) {
                    let _ = record::disable_context(&ctrl, ctx);
                    let _ = ctrl.flush();
                }
            }
            if let Some(h) = self.handle.lock().unwrap_or_else(|e| e.into_inner()).take() {
                let _ = h.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_normalization_and_display() {
        assert_eq!(normalize_binding("VK117", "f6"), "f6");
        assert_eq!(normalize_binding("vk27", "f6"), "esc");
        assert_eq!(normalize_binding("", "f6"), "f6");
        assert_eq!(display_name("f6"), "F6");
        assert_eq!(display_name("mouse:x1"), "Mouse4");
        assert_eq!(display_name("mouse:x2"), "Mouse5");
        assert_eq!(display_name("vk96"), "Num 0");
    }

    #[test]
    fn numeric_conflicts_checked() {
        assert!(conflicts_with_numeric_entry("5"));
        assert!(conflicts_with_numeric_entry("vk97"));
        assert!(conflicts_with_numeric_entry("backspace"));
        assert!(!conflicts_with_numeric_entry("f6"));
        assert!(!conflicts_with_numeric_entry("mouse:x1"));
    }

    #[test]
    fn trigger_listener_records_and_cancels() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev_copy = Arc::clone(&events);
        let listener = TriggerListener::new(
            "f6",
            Arc::new(move |ev| {
                ev_copy.lock().unwrap().push(ev);
            }),
        );

        listener.process_key("f6", true);
        listener.process_key("f6", true); // repeat ignored
        listener.process_key("f6", false);
        listener.process_key("esc", true);

        listener.set_recording(true);
        listener.process_key("esc", true); // cancels recording
        listener.set_recording(true);
        listener.process_key("mouse:x1", true);

        let recorded = events.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec![
                TriggerEvent::Down,
                TriggerEvent::Up,
                TriggerEvent::Panic,
                TriggerEvent::RecordCancelled,
                TriggerEvent::Recorded("mouse:x1".to_string()),
            ]
        );
        assert_eq!(listener.binding(), "mouse:x1");
    }
}
