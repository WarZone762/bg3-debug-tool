use imgui::sys::igGetIO;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{EnumWindows, GetWindow, IsWindowVisible, GW_OWNER},
    },
};

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
        ImGui_ImplWin32_Init(hwnd.0 as _);
        unsafe extern "system" fn subclass_wnd_proc(
            hwnd: HWND,
            umsg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
            _uid_subclass: usize,
            _dwref_data: usize,
        ) -> LRESULT {
            ImGui_ImplWin32_WndProcHandler(hwnd, umsg, wparam, lparam);

            let io = igGetIO();
            if (*io).WantCaptureMouse || (*io).WantCaptureKeyboard {
                return LRESULT(1);
            }

            DefSubclassProc(hwnd, umsg, wparam, lparam)
        }

        SetWindowSubclass(hwnd, Some(subclass_wnd_proc), 1, 0);
    }
}

pub(crate) fn new_frame() {
    unsafe { ImGui_ImplWin32_NewFrame() };
}

#[link(name = "imgui_backends", kind = "static")]
extern "C" {
    fn ImGui_ImplWin32_Init(hwnd: *mut libc::c_void) -> bool;
    fn ImGui_ImplWin32_WndProcHandler(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM)
        -> bool;
    fn ImGui_ImplWin32_NewFrame();
}
