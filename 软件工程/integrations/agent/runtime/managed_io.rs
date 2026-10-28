//! Read-only OS leases, never caller/AssetRef paths. Only authenticated frames
//! transfer ownership of a duplicated handle into the bridge.
#![cfg(windows)]
use super::pipe_os::{Identity, Peer, with_logon_security};
use std::{
    fs::File,
    io::{self, Read, Seek, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::PathBuf,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::{
        DuplicateHandle, GENERIC_READ, GENERIC_WRITE, GetHandleInformation, HANDLE_FLAG_INHERIT,
        INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{
        CREATE_NEW, CreateFileW, DELETE, FILE_ALL_ACCESS, FILE_ATTRIBUTE_TEMPORARY,
        FILE_FLAG_DELETE_ON_CLOSE, FILE_SHARE_DELETE, FILE_SHARE_READ, OPEN_EXISTING,
    },
    System::Threading::GetCurrentProcess,
};

pub fn publish_readonly(bytes: &[u8], peer: &Peer, identity: &Identity) -> io::Result<u64> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let path: PathBuf = std::env::temp_dir().join(format!(
        "superfactory-managed-{}-{nonce}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut created = false;
    let written = with_logon_security(identity, FILE_ALL_ACCESS, |attributes| {
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_DELETE,
                attributes,
                CREATE_NEW,
                FILE_ATTRIBUTE_TEMPORARY,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        created = true;
        let mut file = unsafe { File::from_raw_handle(handle.cast()) };
        file.write_all(bytes)?;
        file.flush()
    });
    if let Err(error) = written {
        if created {
            let _ = std::fs::remove_file(&path);
        }
        return Err(error);
    }
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | DELETE,
            FILE_SHARE_READ | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_DELETE_ON_CLOSE | FILE_ATTRIBUTE_TEMPORARY,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let error = io::Error::last_os_error();
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    let file = unsafe { File::from_raw_handle(handle.cast()) };
    let mut target = ptr::null_mut();
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            file.as_raw_handle().cast(),
            peer.process.0,
            &mut target,
            GENERIC_READ,
            0,
            0,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // The authenticated bridge owns the target now. It closes known leases on
    // consumption; any broken/ambiguous stream terminates that bridge process,
    // letting Windows close even a lease whose frame never arrived completely.
    Ok(target as usize as u64)
}

pub struct ReceivedLease {
    file: File,
    value: u64,
}
impl ReceivedLease {
    /// Caller must first authenticate the IPC frame and acquire the lease only
    /// from its Host-owned attachment, never from MCP arguments or an asset path.
    pub unsafe fn receive(value: u64) -> io::Result<Self> {
        let raw = usize::try_from(value).map_err(io::Error::other)? as *mut std::ffi::c_void;
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid managed lease",
            ));
        }
        let mut flags = 0;
        if unsafe { GetHandleInformation(raw, &mut flags) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_handle(raw) };
        if flags & HANDLE_FLAG_INHERIT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Managed lease must not be inherited",
            ));
        }
        Ok(Self { file, value })
    }

    pub fn read_exact_original(
        mut self,
        announced_value: u64,
        length: u64,
        cancelled: impl Fn() -> bool,
    ) -> io::Result<Vec<u8>> {
        if announced_value != self.value
            || self.file.metadata()?.len() != length
            || self.file.stream_position()? != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Managed lease identity, size or initial position mismatch",
            ));
        }
        let length = usize::try_from(length).map_err(io::Error::other)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(io::Error::other)?;
        let mut block = [0u8; 65536];
        while bytes.len() < length {
            if cancelled() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Managed transfer cancelled; business task state is unchanged",
                ));
            }
            let remaining = block.len().min(length - bytes.len());
            let read = self.file.read(&mut block[..remaining])?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Managed result ended early",
                ));
            }
            bytes.extend_from_slice(&block[..read]);
        }
        if self.file.read(&mut block[..1])? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Managed result has trailing bytes",
            ));
        }
        Ok(bytes)
    }
}
