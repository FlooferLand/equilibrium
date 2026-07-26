use std::ffi::c_void;

pub fn fix_mouse_passthrough(frame: &mut eframe::Frame) {
    #[cfg(windows)] use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = frame.window_handle().unwrap().as_raw();
    match handle {
        #[cfg(windows)]
        RawWindowHandle::Win32(handle) => unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            
            let hwnd = handle.hwnd.get() as *mut c_void;
            
            let style = GetWindowLongA(hwnd as *mut _, GWL_STYLE);
            SetWindowLongA(hwnd as *mut _, GWL_STYLE, style & !(WS_THICKFRAME as i32 | WS_BORDER as i32));
            
            let ex_style = GetWindowLongA(hwnd as *mut _, GWL_EXSTYLE);
            SetWindowLongA(hwnd as *mut _, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED as i32) & !(WS_EX_TRANSPARENT as i32) | WS_EX_TRANSPARENT as i32);
        }
        _ => {}
    }
}

pub fn init() {
    #[cfg(windows)]
    unsafe {
        use std::{fs::OpenOptions, os::windows::io::AsRawHandle};
        use windows_sys::Win32::System::Console::{
            AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE
        };        
        if AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            if let Ok(con) = OpenOptions::new().write(true).open("CONOUT$") {
                SetStdHandle(STD_OUTPUT_HANDLE, con.as_raw_handle() as _);
                std::mem::forget(con);
            }
        }
    }
}
