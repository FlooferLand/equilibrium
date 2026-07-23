use std::ffi::c_void;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub fn fix_mouse_passthrough(frame: &mut eframe::Frame) {
    let handle = frame.window_handle().unwrap().as_raw();
    match handle {
        RawWindowHandle::Win32(handle) => {
            unsafe {
                let hwnd = handle.hwnd.get() as *mut c_void;
                
                let style = GetWindowLongA(hwnd as *mut _, GWL_STYLE);
                SetWindowLongA(hwnd as *mut _, GWL_STYLE, style & !(WS_THICKFRAME as i32 | WS_BORDER as i32));
                
                let ex_style = GetWindowLongA(hwnd as *mut _, GWL_EXSTYLE);
                SetWindowLongA(hwnd as *mut _, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED as i32) & !(WS_EX_TRANSPARENT as i32) | WS_EX_TRANSPARENT as i32);
            }
        }
        _ => {}
    }
}