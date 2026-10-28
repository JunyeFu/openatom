//! Shared native transport primitives. Identity comes only from Windows.
#![cfg(windows)]

use std::{io, mem, os::windows::io::AsRawHandle, ptr};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{
        ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAce, GetLengthSid,
        GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor, SECURITY_ATTRIBUTES,
        SECURITY_DESCRIPTOR, SetSecurityDescriptorDacl, TOKEN_GROUPS, TOKEN_INFORMATION_CLASS,
        TOKEN_QUERY, TOKEN_USER, TokenLogonSid, TokenSessionId, TokenUser,
    },
    Storage::FileSystem::{FILE_GENERIC_READ, FILE_GENERIC_WRITE},
    System::{
        Pipes::{
            GetNamedPipeClientProcessId, GetNamedPipeClientSessionId, GetNamedPipeServerProcessId,
            GetNamedPipeServerSessionId,
        },
        Threading::{
            GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken,
            PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

pub struct ProcessHandle(pub HANDLE);
// Windows process handles may be queried/duplicated/closed from any thread.
unsafe impl Send for ProcessHandle {}
unsafe impl Sync for ProcessHandle {}
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub pid: u32,
    pub session: u32,
    pub user_sid: Vec<u8>,
    pub logon_sid: Vec<u8>,
}

pub struct Peer {
    pub identity: Identity,
    pub process: ProcessHandle,
}

fn denied(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

fn token_bytes(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> io::Result<Vec<usize>> {
    let mut needed = 0;
    unsafe {
        GetTokenInformation(token, class, ptr::null_mut(), 0, &mut needed);
    }
    if needed == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut storage = vec![0usize; (needed as usize).div_ceil(mem::size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token,
            class,
            storage.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(storage)
}

unsafe fn sid_bytes(sid: *mut std::ffi::c_void) -> io::Result<Vec<u8>> {
    let size = unsafe { GetLengthSid(sid) };
    if size == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), size as usize) }.to_vec())
}

fn process_identity(process: HANDLE, pid: u32) -> io::Result<Identity> {
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = ProcessHandle(token);
    let user = token_bytes(token.0, TokenUser)?;
    let user_sid = unsafe { sid_bytes((*(user.as_ptr().cast::<TOKEN_USER>())).User.Sid)? };
    let logon = token_bytes(token.0, TokenLogonSid)?;
    let groups = unsafe { &*(logon.as_ptr().cast::<TOKEN_GROUPS>()) };
    if groups.GroupCount != 1 {
        return Err(denied("An actual interactive logon SID is required"));
    }
    let logon_sid = unsafe { sid_bytes(groups.Groups[0].Sid)? };
    let session = token_bytes(token.0, TokenSessionId)?;
    Ok(Identity {
        pid,
        session: unsafe { *session.as_ptr().cast::<u32>() },
        user_sid,
        logon_sid,
    })
}

pub fn current_identity() -> io::Result<Identity> {
    process_identity(unsafe { GetCurrentProcess() }, unsafe {
        GetCurrentProcessId()
    })
}

pub fn pipe_peer(pipe: &impl AsRawHandle, server_side: bool, local: &Identity) -> io::Result<Peer> {
    let handle = pipe.as_raw_handle() as HANDLE;
    let (mut pid, mut session) = (0, 0);
    let ok = unsafe {
        if server_side {
            GetNamedPipeClientProcessId(handle, &mut pid) != 0
                && GetNamedPipeClientSessionId(handle, &mut session) != 0
        } else {
            GetNamedPipeServerProcessId(handle, &mut pid) != 0
                && GetNamedPipeServerSessionId(handle, &mut session) != 0
        }
    };
    if !ok {
        return Err(io::Error::last_os_error());
    }
    let access =
        PROCESS_QUERY_LIMITED_INFORMATION | if server_side { PROCESS_DUP_HANDLE } else { 0 };
    let process = unsafe { OpenProcess(access, 0, pid) };
    if process.is_null() {
        return Err(io::Error::last_os_error());
    }
    let process = ProcessHandle(process);
    let identity = process_identity(process.0, pid)?;
    if identity.session != session
        || identity.session != local.session
        || identity.user_sid != local.user_sid
        || identity.logon_sid != local.logon_sid
    {
        return Err(denied(
            "Pipe peer must be the actual current user, logon and Windows session",
        ));
    }
    Ok(Peer { identity, process })
}

pub fn create_server(
    endpoint: &str,
    first: bool,
    identity: &Identity,
) -> io::Result<NamedPipeServer> {
    // Explicit current-logon DACL; the remote-client rejection is a separate
    // kernel pipe flag. The name and welcome session are never authorization.
    with_logon_security(
        identity,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        |attributes| unsafe {
            ServerOptions::new()
                .first_pipe_instance(first)
                .reject_remote_clients(true)
                .create_with_security_attributes_raw(endpoint, attributes.cast())
        },
    )
}

pub fn with_logon_security<R>(
    identity: &Identity,
    access: u32,
    action: impl FnOnce(*mut SECURITY_ATTRIBUTES) -> io::Result<R>,
) -> io::Result<R> {
    let length =
        mem::size_of::<ACL>() + mem::size_of::<ACCESS_ALLOWED_ACE>() - 4 + identity.logon_sid.len();
    let mut acl = vec![0usize; length.div_ceil(mem::size_of::<usize>())];
    let acl_ptr = acl.as_mut_ptr().cast::<ACL>();
    let mut descriptor: SECURITY_DESCRIPTOR = unsafe { mem::zeroed() };
    let ok = unsafe {
        InitializeAcl(acl_ptr, length as u32, ACL_REVISION) != 0
            && AddAccessAllowedAce(
                acl_ptr,
                ACL_REVISION,
                access,
                identity.logon_sid.as_ptr() as _,
            ) != 0
            && InitializeSecurityDescriptor((&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(), 1)
                != 0
            && SetSecurityDescriptorDacl(
                (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(),
                1,
                acl_ptr,
                0,
            ) != 0
    };
    if !ok {
        return Err(io::Error::last_os_error());
    }
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(),
        bInheritHandle: 0,
    };
    action(&mut attributes)
}
