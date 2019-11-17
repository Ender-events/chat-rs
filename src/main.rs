mod epoll;
use epoll::{Epoll, EpollResult};
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::prelude::*;
use std::net::TcpListener;
use std::os::unix::io::AsRawFd;

#[macro_use]
extern crate bitflags;

type ClientStorage = HashMap<i32, std::net::TcpStream>;

fn main() {
    let mut streams = HashMap::new();
    let epoll = Epoll::create().unwrap();

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();

    epoll
        .ctl_add(
            &listener,
            epoll::Events::EPOLLIN,
            listener.as_raw_fd().try_into().unwrap(),
        )
        .unwrap();
    let mut events = EpollResult::create(10);
    loop {
        epoll.wait(-1, &mut events).unwrap();
        for (_events, data) in events.iter() {
            if data == listener.as_raw_fd().try_into().unwrap() {
                accept(&listener, &epoll, &mut streams);
            } else {
                let mut stream =
                    streams.get(&data.try_into().unwrap()).unwrap();
                let mut buffer = [0; 512];
                let nb_read = stream.read(&mut buffer).unwrap();
                if nb_read == 0 {
                    streams.remove(&data.try_into().unwrap());
                    continue;
                }
                let msg = &buffer[0..nb_read];
                stream.write(msg).unwrap();
            }
        }
    }
}

fn accept(listener: &TcpListener, epoll: &Epoll, streams: &mut ClientStorage) {
    let stream = listener.accept().unwrap().0;
    stream.set_nonblocking(true).unwrap();
    epoll
        .ctl_add(
            &stream,
            epoll::Events::EPOLLIN,
            stream.as_raw_fd().try_into().unwrap(),
        )
        .unwrap();
    streams.insert(stream.as_raw_fd(), stream);
}
