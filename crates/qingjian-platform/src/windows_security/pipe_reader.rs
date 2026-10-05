use super::{PeekNamedPipe, ptr};
use std::io;
/// 客户端握手读取具有超时，伪装的同用户服务端不能无限阻塞应用 UI 线程。
pub struct PipeReader<'a> {
    file: &'a mut std::fs::File,
    deadline: std::time::Instant,
}

impl<'a> PipeReader<'a> {
    pub fn new(file: &'a mut std::fs::File, timeout: std::time::Duration) -> Self {
        Self {
            file,
            deadline: std::time::Instant::now() + timeout,
        }
    }
}

impl std::io::Read for PipeReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        use std::os::windows::io::AsRawHandle;
        if bytes.is_empty() {
            return Ok(0);
        }
        loop {
            let mut available = 0;
            if unsafe {
                PeekNamedPipe(
                    self.file.as_raw_handle(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut available,
                    ptr::null_mut(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            if available > 0 {
                let length = bytes.len().min(available as usize);
                return std::io::Read::read(self.file, &mut bytes[..length]);
            }
            if std::time::Instant::now() >= self.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "pipe handshake timeout",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}
