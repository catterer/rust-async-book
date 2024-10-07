use crate::ffi;
use std::{
    io::{self, Result},
    net::TcpStream,
    os::fd::AsRawFd,
};

type Events = Vec<ffi::Event>;

pub struct Poll {
    registry: Registry,
}

impl Poll {
    pub fn new() -> Result<Self> {
        let res = unsafe { ffi::epoll_create(1) };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self{
            registry: Registry{ raw_fd: res }
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn poll(&mut self, events: &mut Events, timeout_ms: Option<i32>) -> Result<()> {
        let r = unsafe { ffi::epoll_wait(self.registry.raw_fd, events.as_mut_ptr(), events.capacity() as i32, timeout_ms.unwrap_or(-1)) };
        if r < 0 {
            return Err(io::Error::last_os_error());
        }

        unsafe { events.set_len(r as usize) };
        Ok(())
    }
}

pub struct Registry {
    raw_fd: i32,
}

impl Registry {
    pub fn register(&self, source: &TcpStream, token: usize, interests: i32) -> Result<()> {
        let mut event = ffi::Event {
            events: interests as u32,
            epoll_data: token,
        };
        let op = ffi::EPOLL_CTL_ADD;
        let r = unsafe {
            ffi::epoll_ctl(self.raw_fd, op, source.as_raw_fd(), &mut event)
        };

        if r < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        let r = unsafe { ffi::close(self.raw_fd) };
        if r < 0 {
            let e = io::Error::last_os_error();
            eprintln!("ERROR: {e:?}");
        }
    }
}
