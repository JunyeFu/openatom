use std::{cell::Cell, io, ptr};
use windows_sys::Win32::{
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        CBT_CREATEWNDW, CallNextHookEx, GetClassNameW, HCBT_CREATEWND, HHOOK, SetWindowsHookExW,
        UnhookWindowsHookEx, WH_CBT,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_origin_for_200_percent_screen_to_right_of_100_percent_primary() {
        // Primary physical range [0,1920), secondary [1920,3840).
        // At 200%, a global logical x of 984 also falls inside the primary.
        assert_eq!(creation_origin(1920, 0), (1968, 48));
        assert_eq!(creation_origin(-1920, 0), (-1872, 48));
        use windows_sys::Win32::{
            Foundation::RECT,
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, IsWindowVisible,
                RegisterClassW, WNDCLASSW, WS_POPUP,
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
            let hook = CreationHook::install(creation_origin(1920, 0)).unwrap();
            let window = CreateWindowExW(
                0,
                name.as_ptr(),
                name.as_ptr(),
                WS_POPUP,
                984,
                24,
                200,
                100,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            );
            assert!(!window.is_null());
            let mut rect = RECT::default();
            let read = GetWindowRect(window, &mut rect);
            let visible = IsWindowVisible(window);
            DestroyWindow(window);
            assert!(hook.applied());
            assert_ne!(read, 0);
            assert_eq!(visible, 0);
            assert_eq!((rect.left, rect.top), (1968, 48));
        }
    }
}
