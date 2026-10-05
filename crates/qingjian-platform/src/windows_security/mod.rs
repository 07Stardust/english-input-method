//! Windows 用户/登录会话身份与 DPAPI。失败时不降级为开放权限或明文存储。

mod blob;
mod identity;
mod pipe_reader;
use blob::Blob;
pub use identity::Identity;
pub use pipe_reader::PipeReader;
use std::ffi::c_void;
use std::io;
use std::ptr;

type Handle = *mut c_void;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
    fn GetTokenInformation(
        token: Handle,
        class: u32,
        data: *mut c_void,
        length: u32,
        needed: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *mut c_void, text: *mut *mut u16) -> i32;
    fn GetSecurityInfo(
        handle: Handle,
        kind: u32,
        information: u32,
        owner: *mut *mut c_void,
        group: Handle,
        dacl: Handle,
        sacl: Handle,
        descriptor: *mut Handle,
    ) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> Handle;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
    fn CloseHandle(handle: Handle) -> i32;
    fn LocalFree(memory: Handle) -> Handle;
    fn GetNamedPipeClientProcessId(pipe: Handle, pid: *mut u32) -> i32;
    fn GetNamedPipeServerProcessId(pipe: Handle, pid: *mut u32) -> i32;
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
    fn PeekNamedPipe(
        pipe: Handle,
        buffer: Handle,
        size: u32,
        read: *mut u32,
        available: *mut u32,
        remaining: *mut u32,
    ) -> i32;
}

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        input: *const Blob,
        description: *const u16,
        entropy: *const Blob,
        reserved: Handle,
        prompt: Handle,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *const Blob,
        description: *mut *mut u16,
        entropy: *const Blob,
        reserved: Handle,
        prompt: Handle,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
}

fn identity(process: Handle) -> io::Result<Identity> {
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(process, 8, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        let mut needed = 0;
        unsafe { GetTokenInformation(token, 1, ptr::null_mut(), 0, &mut needed) };
        if needed == 0 || needed > 65536 {
            return Err(io::Error::other("invalid token size"));
        }
        // usize 分配保证 TOKEN_USER 内的指针对齐。
        let mut data = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        if unsafe { GetTokenInformation(token, 1, data.as_mut_ptr().cast(), needed, &mut needed) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
        let sid = data[0] as *mut c_void;
        let mut text = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut length = 0;
        while unsafe { *text.add(length) } != 0 {
            length += 1;
        }
        let sid = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) });
        unsafe { LocalFree(text.cast()) };
        let mut session = 0u32;
        if unsafe { GetTokenInformation(token, 12, (&raw mut session).cast(), 4, &mut needed) } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Identity { sid, session })
    })();
    unsafe { CloseHandle(token) };
    result
}

pub fn current_identity() -> io::Result<Identity> {
    identity(unsafe { GetCurrentProcess() })
}

pub fn pipe_name() -> io::Result<String> {
    let owner = current_identity()?;
    Ok(format!(
        r"\\.\pipe\EnglishInputMethod-{}-{}",
        owner.sid, owner.session
    ))
}

pub fn pipe_sddl() -> io::Result<String> {
    let owner = current_identity()?;
    // AppContainer 仅获得读写权，不获得创建管道实例权；连接后仍验证账户和登录会话。
    Ok(format!(
        "O:{0}D:P(A;;GA;;;{0})(A;;0x12019B;;;AC)(A;;0x12019B;;;S-1-15-2-2)S:(ML;;NW;;;LW)",
        owner.sid
    ))
}

/// 管道句柄必须在本次调用期间有效。
pub fn verify_peer(pipe: isize, server_side: bool) -> io::Result<()> {
    let mut pid = 0;
    let ok = unsafe {
        if server_side {
            GetNamedPipeClientProcessId(pipe as Handle, &mut pid)
        } else {
            GetNamedPipeServerProcessId(pipe as Handle, &mut pid)
        }
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    if !server_side {
        // AppContainer 无需打开中完整性服务端的进程令牌；使用内核管道所有者与服务端登录会话核验。
        let mut owner_sid = ptr::null_mut();
        let mut descriptor = ptr::null_mut();
        let status = unsafe {
            GetSecurityInfo(
                pipe as Handle,
                6,
                1,
                &mut owner_sid,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let owner = (|| {
            let mut text = ptr::null_mut();
            if owner_sid.is_null() || unsafe { ConvertSidToStringSidW(owner_sid, &mut text) } == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut length = 0;
            while unsafe { *text.add(length) } != 0 {
                length += 1;
            }
            let sid = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) });
            unsafe { LocalFree(text.cast()) };
            let mut session = 0;
            if unsafe { ProcessIdToSessionId(pid, &mut session) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Identity { sid, session })
        })();
        unsafe { LocalFree(descriptor) };
        if owner? != current_identity()? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "pipe server identity mismatch",
            ));
        }
        return Ok(());
    }
    let process = unsafe { OpenProcess(0x1000, 0, pid) };
    if process.is_null() {
        return Err(io::Error::last_os_error());
    }
    let peer = identity(process);
    unsafe { CloseHandle(process) };
    if peer? != current_identity()? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "pipe peer identity mismatch",
        ));
    }
    Ok(())
}

fn crypt(bytes: &[u8], protect: bool) -> io::Result<Vec<u8>> {
    let input = Blob {
        length: u32::try_from(bytes.len()).map_err(|_| io::Error::other("secret too large"))?,
        data: bytes.as_ptr().cast_mut(),
    };
    let mut output = Blob {
        length: 0,
        data: ptr::null_mut(),
    };
    let ok = unsafe {
        if protect {
            CryptProtectData(
                &input,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                1,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                1,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.data, output.length as usize) }.to_vec();
    unsafe { LocalFree(output.data.cast()) };
    Ok(result)
}

pub fn protect_secret(bytes: &[u8]) -> io::Result<Vec<u8>> {
    crypt(bytes, true)
}

pub fn unprotect_secret(bytes: &[u8]) -> io::Result<Vec<u8>> {
    crypt(bytes, false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn dpapi_round_trip_and_tamper() {
        let encrypted = super::protect_secret(b"test-only-not-a-real-key").unwrap();
        assert_ne!(encrypted, b"test-only-not-a-real-key");
        assert_eq!(
            super::unprotect_secret(&encrypted).unwrap(),
            b"test-only-not-a-real-key"
        );
        let mut damaged = encrypted;
        damaged[0] ^= 0xff;
        assert!(super::unprotect_secret(&damaged).is_err());
    }
}
