//! `cfg(windows)`：打开 Server 的命名管道，得到一条双工流交给 [`EngineClient`](super::EngineClient)。

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::ERROR_PIPE_BUSY;
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows::core::HSTRING;

use qingjian_platform::protocol::{Hello, PROTOCOL_VERSION, Welcome, read_message, write_message};

/// 连好的命名管道。对端关闭时读到 EOF；`flush` 是空操作（管道上 `FlushFileBuffers` 会阻塞到对端读完）。
pub type PipeStream = File;

/// 实例都被占着时等一个可用实例的超时（毫秒）与重试次数。要短：这里在应用 UI 线程上，等久了 TSF 看门狗会切走输入法。
const BUSY_WAIT_MS: u32 = 300;
const BUSY_RETRIES: u32 = 1;

pub fn connect_default() -> io::Result<PipeStream> {
    connect(&qingjian_platform::windows_security::pipe_name()?)
}

pub fn connect(name: &str) -> io::Result<PipeStream> {
    let mut attempts = 0;
    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .access_mode(0x12019B)
            .share_mode(0)
            .open(name)
        {
            Err(error)
                if error.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32)
                    && attempts < BUSY_RETRIES =>
            {
                attempts += 1;
                let _ = unsafe { WaitNamedPipeW(&HSTRING::from(name), BUSY_WAIT_MS) };
            }
            Ok(mut stream) => {
                qingjian_platform::windows_security::verify_peer(
                    stream.as_raw_handle() as isize,
                    false,
                )?;
                write_message(
                    &mut stream,
                    &Hello {
                        protocol: PROTOCOL_VERSION,
                    },
                )
                .map_err(io::Error::other)?;
                let mut reader = qingjian_platform::windows_security::PipeReader::new(
                    &mut stream,
                    std::time::Duration::from_millis(300),
                );
                let welcome = read_message::<_, Welcome>(&mut reader).map_err(io::Error::other)?;
                if !matches!(
                    welcome,
                    Some(Welcome {
                        protocol: PROTOCOL_VERSION
                    })
                ) {
                    return Err(io::Error::other("协议版本不兼容，请重启使用输入法的应用"));
                }
                return Ok(stream);
            }
            Err(error) => return Err(error),
        }
    }
}
