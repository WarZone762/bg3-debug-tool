use std::{collections::HashMap, mem, sync::Mutex};

use egui::{
    Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, ViewportId, ViewportInfo,
};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM},
    System::SystemServices::{MK_CONTROL, MK_SHIFT},
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
            VK_ESCAPE, VK_HOME, VK_INSERT, VK_LEFT, VK_LSHIFT, VK_NEXT, VK_PRIOR, VK_RETURN,
            VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
        },
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{
            EnumWindows, GetClientRect, GetForegroundWindow, GetWindow, IsWindowVisible, GW_OWNER,
            KF_REPEAT, WHEEL_DELTA, WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP,
            WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN,
            WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDBLCLK, WM_XBUTTONDOWN,
            WM_XBUTTONUP, XBUTTON1, XBUTTON2,
        },
    },
};

use crate::info;

static INPUT_MANAGER: Mutex<InputManager> = Mutex::new(InputManager::new(HWND(0)));

pub(crate) fn init() {
    unsafe extern "system" fn is_main(handle: HWND, lparam: LPARAM) -> BOOL {
        if GetWindow(handle, GW_OWNER) == HWND::default() && IsWindowVisible(handle).as_bool() {
            *(lparam.0 as *mut HWND) = handle;
            false.into()
        } else {
            true.into()
        }
    }

    let mut hwnd = HWND(0);
    unsafe {
        let _ = EnumWindows(Some(is_main), LPARAM(&mut hwnd as *mut _ as _));
        INPUT_MANAGER.lock().unwrap().hwnd = hwnd;
        unsafe extern "system" fn subclass_wnd_proc(
            hwnd: HWND,
            umsg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
            _uid_subclass: usize,
            _dwref_data: usize,
        ) -> LRESULT {
            // if (*io).WantCaptureMouse || (*io).WantCaptureKeyboard {
            // return LRESULT(1);
            // }

            INPUT_MANAGER.lock().unwrap().process(umsg, wparam.0, lparam.0);

            DefSubclassProc(hwnd, umsg, wparam, lparam)
        }

        SetWindowSubclass(hwnd, Some(subclass_wnd_proc), 1, 0);
    }
}

pub(crate) fn new_frame() -> Result<RawInput> {
    INPUT_MANAGER.lock().unwrap().collect_input()
}

pub struct InputManager {
    hwnd: HWND,
    events: Vec<Event>,
    modifiers: Option<Modifiers>,
}

#[derive(Debug)]
#[repr(u8)]
pub enum InputResult {
    Unknown,
    MouseMove,
    MouseLeft,
    MouseRight,
    MouseMiddle,
    Character,
    Scroll,
    Zoom,
    Key,
}

type Result<T> = std::result::Result<T, windows::core::Error>;

impl InputResult {
    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.is_unknown()
    }

    #[inline]
    pub fn is_unknown(&self) -> bool {
        matches!(*self, InputResult::Unknown)
    }
}

impl InputManager {
    pub const fn new(hwnd: HWND) -> Self {
        Self { hwnd, events: vec![], modifiers: None }
    }

    pub fn collect_input(&mut self) -> Result<RawInput> {
        Ok(RawInput {
            viewport_id: ViewportId::ROOT,
            viewports: HashMap::from_iter([(ViewportId::ROOT, ViewportInfo {
                parent: None,
                title: None,
                events: vec![],
                native_pixels_per_point: Some(1.0),
                monitor_size: Some(self.get_screen_size()),
                inner_rect: Some(self.get_screen_rect()),
                outer_rect: None,
                minimized: None,
                maximized: None,
                fullscreen: None,
                focused: Some(true),
            })]),
            screen_rect: Some(self.get_screen_rect()),
            max_texture_side: None,
            time: None,
            // time: Some(Self::get_system_time()?),
            predicted_dt: 1.0 / 60.0,
            modifiers: self.modifiers.unwrap_or_default(),
            events: mem::take(&mut self.events),
            hovered_files: vec![],
            dropped_files: vec![],
            focused: true,
        })
    }

    pub fn process(&mut self, umsg: u32, wparam: usize, lparam: isize) -> InputResult {
        match umsg {
            WM_MOUSEMOVE => {
                self.alter_modifiers(get_mouse_modifiers(wparam));

                self.events.push(Event::PointerMoved(get_pos(lparam)));
                InputResult::MouseMove
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers,
                });
                InputResult::MouseLeft
            }
            WM_LBUTTONUP => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers,
                });
                InputResult::MouseLeft
            }
            WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: PointerButton::Secondary,
                    pressed: true,
                    modifiers,
                });
                InputResult::MouseRight
            }
            WM_RBUTTONUP => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: PointerButton::Secondary,
                    pressed: false,
                    modifiers,
                });
                InputResult::MouseRight
            }
            WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: PointerButton::Middle,
                    pressed: true,
                    modifiers,
                });
                InputResult::MouseMiddle
            }
            WM_MBUTTONUP => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: PointerButton::Middle,
                    pressed: false,
                    modifiers,
                });
                InputResult::MouseMiddle
            }
            WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: if (wparam as u32) >> 16 & (XBUTTON1 as u32) != 0 {
                        PointerButton::Extra1
                    } else if (wparam as u32) >> 16 & (XBUTTON2 as u32) != 0 {
                        PointerButton::Extra2
                    } else {
                        unreachable!()
                    },
                    pressed: true,
                    modifiers,
                });
                InputResult::MouseMiddle
            }
            WM_XBUTTONUP => {
                let modifiers = get_mouse_modifiers(wparam);
                self.alter_modifiers(modifiers);

                self.events.push(Event::PointerButton {
                    pos: get_pos(lparam),
                    button: if (wparam as u32) >> 16 & (XBUTTON1 as u32) != 0 {
                        PointerButton::Extra1
                    } else if (wparam as u32) >> 16 & (XBUTTON2 as u32) != 0 {
                        PointerButton::Extra2
                    } else {
                        unreachable!()
                    },
                    pressed: false,
                    modifiers,
                });
                InputResult::MouseMiddle
            }
            WM_CHAR => {
                if let Some(ch) = char::from_u32(wparam as _) {
                    if !ch.is_control() {
                        self.events.push(Event::Text(ch.into()));
                    }
                }
                InputResult::Character
            }
            WM_MOUSEWHEEL => {
                self.alter_modifiers(get_mouse_modifiers(wparam));

                let delta = (wparam >> 16) as i16 as f32 * 10. / WHEEL_DELTA as f32;

                if wparam & MK_CONTROL.0 as usize != 0 {
                    self.events.push(Event::Zoom(if delta > 0. { 1.5 } else { 0.5 }));
                    InputResult::Zoom
                } else {
                    self.events.push(Event::Scroll(Vec2::new(0., delta)));
                    InputResult::Scroll
                }
            }
            WM_MOUSEHWHEEL => {
                self.alter_modifiers(get_mouse_modifiers(wparam));

                let delta = (wparam >> 16) as i16 as f32 * 10. / WHEEL_DELTA as f32;

                if wparam & MK_CONTROL.0 as usize != 0 {
                    self.events.push(Event::Zoom(if delta > 0. { 1.5 } else { 0.5 }));
                    InputResult::Zoom
                } else {
                    self.events.push(Event::Scroll(Vec2::new(delta, 0.)));
                    InputResult::Scroll
                }
            }
            msg @ (WM_KEYDOWN | WM_SYSKEYDOWN) => {
                let modifiers = get_key_modifiers(msg);
                self.modifiers = Some(modifiers);

                if let Some(key) = get_key(wparam) {
                    if key == Key::V && modifiers.ctrl {
                        // if let Some(clipboard) = get_clipboard_text() {
                        //     self.events.push(Event::Text(clipboard));
                        // }
                    }

                    if key == Key::C && modifiers.ctrl {
                        self.events.push(Event::Copy);
                    }

                    if key == Key::X && modifiers.ctrl {
                        self.events.push(Event::Cut);
                    }

                    self.events.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: lparam & (KF_REPEAT as isize) > 0,
                        modifiers,
                    });
                }
                InputResult::Key
            }
            msg @ (WM_KEYUP | WM_SYSKEYUP) => {
                let modifiers = get_key_modifiers(msg);
                self.modifiers = Some(modifiers);

                if let Some(key) = get_key(wparam) {
                    self.events.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers,
                    });
                }
                InputResult::Key
            }
            _ => InputResult::Unknown,
        }
    }

    fn alter_modifiers(&mut self, new: Modifiers) {
        if let Some(old) = self.modifiers.as_mut() {
            *old = new;
        }
    }

    // pub fn get_system_time() -> Result<f64> {
    //     let mut time = 0;
    //     unsafe {
    //         NtQuerySystemTime(&mut time)?;
    //     }
    //
    //     Ok((time as f64) / 10_000_000.)
    // }

    #[inline]
    pub fn get_screen_size(&self) -> Vec2 {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }

        Vec2::new((rect.right - rect.left) as _, (rect.bottom - rect.top) as _)
    }

    #[inline]
    pub fn get_screen_rect(&self) -> Rect {
        Rect { min: Pos2::ZERO, max: self.get_screen_size().to_pos2() }
    }
}

fn get_pos(lparam: isize) -> Pos2 {
    let x = (lparam & 0xFFFF) as i16 as f32;
    let y = (lparam >> 16 & 0xFFFF) as i16 as f32;

    Pos2::new(x, y)
}

fn get_mouse_modifiers(wparam: usize) -> Modifiers {
    Modifiers {
        alt: false,
        ctrl: (wparam & MK_CONTROL.0 as usize) != 0,
        shift: (wparam & MK_SHIFT.0 as usize) != 0,
        mac_cmd: false,
        command: (wparam & MK_CONTROL.0 as usize) != 0,
    }
}

fn get_key_modifiers(msg: u32) -> Modifiers {
    let ctrl = unsafe { GetAsyncKeyState(VK_CONTROL.0 as _) != 0 };
    let shift = unsafe { GetAsyncKeyState(VK_LSHIFT.0 as _) != 0 };

    Modifiers { alt: msg == WM_SYSKEYDOWN, mac_cmd: false, command: ctrl, shift, ctrl }
}

fn get_key(wparam: usize) -> Option<Key> {
    match wparam {
        0x30..=0x39 => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x1F)) },
        0x41..=0x5A => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x26)) },
        0x70..=0x83 => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x3B)) },
        _ => match VIRTUAL_KEY(wparam as u16) {
            VK_DOWN => Some(Key::ArrowDown),
            VK_LEFT => Some(Key::ArrowLeft),
            VK_RIGHT => Some(Key::ArrowRight),
            VK_UP => Some(Key::ArrowUp),
            VK_ESCAPE => Some(Key::Escape),
            VK_TAB => Some(Key::Tab),
            VK_BACK => Some(Key::Backspace),
            VK_RETURN => Some(Key::Enter),
            VK_SPACE => Some(Key::Space),
            VK_INSERT => Some(Key::Insert),
            VK_DELETE => Some(Key::Delete),
            VK_HOME => Some(Key::Home),
            VK_END => Some(Key::End),
            VK_PRIOR => Some(Key::PageUp),
            VK_NEXT => Some(Key::PageDown),
            _ => None,
        },
    }
}

// fn get_clipboard_text() -> Option<String> {
//     clipboard_win::get_clipboard(clipboard_win::formats::Unicode).ok()
// }
