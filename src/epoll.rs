use std::convert::TryInto;
use std::io::Error;
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

pub struct Epoll {
    fd: RawFd,
}

pub struct EpollResult {
    events: std::vec::Vec<libc::epoll_event>,
    nfds: usize,
}

bitflags! {
    pub struct Events: u32 {
        const EPOLLIN = libc::EPOLLIN as u32;
        const EPOLLOUT = libc::EPOLLOUT as u32;
        const EPOLLRDHUP = libc::EPOLLRDHUP as u32;
        const EPOLLPRI = libc::EPOLLPRI as u32;
        const EPOLLERR = libc::EPOLLERR as u32;
        const EPOLLHUP = libc::EPOLLHUP as u32;
        const EPOLLET = libc::EPOLLET as u32;
    }
}

impl Epoll {
    pub fn create() -> Result<Epoll, Error> {
        let fd = unsafe { libc::epoll_create1(0) };
        if fd == -1 {
            Err(Error::last_os_error())
        } else {
            Ok(Epoll { fd })
        }
    }

    pub fn ctl_add<T: AsRawFd>(
        &self,
        stream: &T,
        event: Events,
        data: usize,
    ) -> Result<(), Error> {
        self.ctl(stream, libc::EPOLL_CTL_ADD, event, data)
    }

    pub fn ctl_mod<T: AsRawFd>(
        &self,
        stream: &T,
        event: Events,
        data: usize,
    ) -> Result<(), Error> {
        self.ctl(stream, libc::EPOLL_CTL_MOD, event, data)
    }

    pub fn ctl<T: AsRawFd>(
        &self,
        stream: &T,
        op: i32,
        event: Events,
        data: usize,
    ) -> Result<(), Error> {
        let fd = stream.as_raw_fd();
        let mut ev = libc::epoll_event {
            events: event.bits(),
            u64: data as u64,
        };
        let rv = unsafe { libc::epoll_ctl(self.fd, op, fd, &mut ev as *mut _) };
        if rv == -1 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn wait(
        &self,
        timeout: i32,
        events: &mut EpollResult,
    ) -> Result<usize, Error> {
        let nfds = unsafe {
            libc::epoll_wait(
                self.fd,
                events.events.as_mut_ptr(),
                events.events.len() as i32,
                timeout,
            )
        };
        if nfds == -1 {
            Err(Error::last_os_error())
        } else {
            events.nfds = nfds.try_into().unwrap();
            Ok(events.nfds)
        }
    }
}

impl EpollResult {
    pub fn create(max_events: usize) -> EpollResult {
        let events = vec![
            unsafe {
                MaybeUninit::<libc::epoll_event>::uninit().assume_init()
            };
            max_events
        ];
        EpollResult { events, nfds: 0 }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (Events, usize)> + 'a {
        self.events[0..self.nfds]
            .iter()
            .map(|ev| (Events::from_bits_truncate(ev.events), ev.u64 as usize))
    }
}
