//! 同步管道的有界读取：只读已经到达的字节，半帧超时断连。

use std::fs::File;
use std::io::{self, Read};
use std::os::windows::io::AsRawHandle;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Pipes::PeekNamedPipe;

pub(super) struct DeadlineReader<'a> {
    stream: &'a mut File,

    deadline: Instant,
}

impl<'a> DeadlineReader<'a> {
    pub fn new(stream: &'a mut File, timeout: Duration) -> Self {
        Self {
            stream,
            deadline: Instant::now() + timeout,
        }
    }
}

impl Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            let mut available = 0;
            unsafe {
                PeekNamedPipe(
                    HANDLE(self.stream.as_raw_handle()),
                    None,
                    0,
                    None,
                    Some(&mut available),
                    None,
                )
            }
            .map_err(io::Error::other)?;
            if available > 0 {
                // 第一批字节到达后，不允许半帧占用连接超过两秒。
                self.deadline = self.deadline.min(Instant::now() + Duration::from_secs(2));
                let size = buffer.len().min(available as usize);
                return self.stream.read(&mut buffer[..size]);
            }
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "pipe read timeout"));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
