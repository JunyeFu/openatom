use std::{cell::Cell, io, ptr};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        CBT_CREATEWNDW, CallNextHookEx, GetClassNameW, GetClientRect, GetWindowRect,
        HCBT_CREATEWND, HHOOK, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
        SetWindowsHookExW, UnhookWindowsHookEx, WH_CBT,
    },
};

thread_local! { static ORIGIN: Cell<Option<(i32, i32)>> = const { Cell::new(None) }; }

// WH_CBT receives the native CREATESTRUCT before Windows creates the HWND on screen.
unsafe extern "system" fn before_create(code: i32, hwnd: usize, data: isize) -> isize {
    if code == HCBT_CREATEWND as i32 {
        let mut class = [0u16; 64];
        // HWND and CBT_CREATEWNDW are supplied by Windows for this callback only.
        let len = unsafe { GetClassNameW(hwnd as _, class.as_mut_ptr(), class.len() as i32) };
        if class[..len as usize]
            .iter()
            .copied()
            .eq("Tauri Window".encode_utf16())
        {
            if let Some((x, y)) = ORIGIN.with(Cell::take) {
                let create = unsafe { &mut *(*(data as *mut CBT_CREATEWNDW)).lpcs };
                create.x = x;
                create.y = y;
            }
        }
    }
    unsafe { CallNextHookEx(ptr::null_mut(), code, hwnd, data) }
}

pub(super) struct CreationHook(HHOOK);

impl CreationHook {
    pub(super) fn install(origin: (i32, i32)) -> io::Result<Self> {
        let hook = unsafe {
            SetWindowsHookExW(
                WH_CBT,
                Some(before_create),
                ptr::null_mut(),
                GetCurrentThreadId(),
            )
        };
        if hook.is_null() {
            return Err(io::Error::last_os_error());
        }
        ORIGIN.with(|slot| slot.set(Some(origin)));
        Ok(Self(hook))
    }

    pub(super) fn applied(&self) -> bool {
        ORIGIN.with(|slot| slot.get().is_none())
    }
}

impl Drop for CreationHook {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.0);
        }
        ORIGIN.with(|slot| slot.set(None));
    }
}

pub(super) fn creation_origin(left: i32, top: i32) -> (i32, i32) {
    (left + 48, top + 48)
}

// The hidden HWND has already been placed on the target monitor. Both rectangles are now
// measured at that HWND's DPI, including its actual title bar and frame.
pub(super) fn fit_hidden_client(hwnd: HWND, work_width: u32, work_height: u32) -> io::Result<()> {
    let width = work_width.saturating_sub(96).max(1) as i32;
    let height = work_height.saturating_sub(128).max(1) as i32;
    let mut outer = RECT::default();
    let mut client = RECT::default();
    unsafe {
        if GetWindowRect(hwnd, &mut outer) == 0 || GetClientRect(hwnd, &mut client) == 0 {
            return Err(io::Error::last_os_error());
        }
        let frame_width = (outer.right - outer.left) - (client.right - client.left);
        let frame_height = (outer.bottom - outer.top) - (client.bottom - client.top);
        if SetWindowPos(
            hwnd,
            ptr::null_mut(),
            0,
            0,
            width + frame_width,
            height + frame_height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        ) == 0
            || GetClientRect(hwnd, &mut client) == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    if (client.right - client.left, client.bottom - client.top) != (width, height) {
        return Err(io::Error::other(
            "Hidden window client size differs from the target work area",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_window_uses_target_physical_origin_and_client_size_in_both_dpi_directions() {
        // Primary physical range [0,1920), secondary [1920,3840).
        // At 200%, a global logical x of 984 also falls inside the primary.
        assert_eq!(creation_origin(1920, 0), (1968, 48));
        assert_eq!(creation_origin(-1920, 0), (-1872, 48));
        use windows_sys::Win32::{
            Foundation::RECT,
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, IsWindowVisible,
                RegisterClassW, WNDCLASSW, WS_OVERLAPPEDWINDOW,
            },
        };
        let name: Vec<u16> = "Tauri Window".encode_utf16().chain(Some(0)).collect();
        // Exercise the HWND creation path with the ambiguous logical coordinate from review.
        unsafe {
            let instance = GetModuleHandleW(ptr::null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(DefWindowProcW),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..Default::default()
            };
            RegisterClassW(&class);
            for (primary_percent, secondary_percent) in [(100, 200), (200, 100)] {
                let hook = CreationHook::install(creation_origin(1920, 0)).unwrap();
                let window = CreateWindowExW(
                    0,
                    name.as_ptr(),
                    name.as_ptr(),
                    WS_OVERLAPPEDWINDOW,
                    984,
                    24,
                    1824 * primary_percent / secondary_percent,
                    912 * primary_percent / secondary_percent,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    instance,
                    ptr::null(),
                );
                assert!(!window.is_null());
                fit_hidden_client(window, 1920, 1040).unwrap();
                let mut rect = RECT::default();
                let mut client = RECT::default();
                assert_ne!(GetWindowRect(window, &mut rect), 0);
                assert_ne!(GetClientRect(window, &mut client), 0);
                let visible = IsWindowVisible(window);
                DestroyWindow(window);
                assert!(hook.applied());
                assert_eq!(visible, 0);
                assert_eq!((rect.left, rect.top), (1968, 48));
                assert_eq!((client.right, client.bottom), (1824, 912));
            }
        }
    }
}
