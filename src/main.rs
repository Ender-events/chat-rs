mod client;
mod epoll;
use client::Client;
use epoll::{Epoll, EpollResult};
use std::collections::HashMap;
use std::convert::TryInto;
use std::net::TcpListener;
use std::os::unix::io::AsRawFd;

#[macro_use]
extern crate bitflags;

type ClientStorage = HashMap<i32, Client>;

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
        for (event, data) in events.iter() {
            if data == listener.as_raw_fd().try_into().unwrap() {
                accept(&listener, &epoll, &mut streams);
            } else if event.contains(epoll::Events::EPOLLIN) {
                read_event(data.try_into().unwrap(), &epoll, &mut streams);
            } else if event.contains(epoll::Events::EPOLLOUT) {
                let client =
                    streams.get_mut(&data.try_into().unwrap()).unwrap();
                write_event(client, data, &epoll);
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
    streams.insert(stream.as_raw_fd(), Client::new(stream));
}

fn read_event(data: i32, epoll: &Epoll, streams: &mut ClientStorage) {
    let client = streams.get_mut(&data).unwrap();
    let nb_read = client.read();
    if nb_read == 0 {
        streams.remove(&data);
        return;
    }
    let msg = client.flush_input();
    for (_, client2) in
        streams.iter_mut().filter(|(&key, _)| key == data as i32)
    {
        client2.send_message(&msg);
        epoll
            .ctl_mod(
                &client2.stream,
                epoll::Events::EPOLLIN | epoll::Events::EPOLLOUT,
                data.try_into().unwrap(),
            )
            .unwrap();
    }
}

fn write_event(client: &mut Client, data: usize, epoll: &Epoll) {
    if client.write() {
        epoll
            .ctl_mod(&client.stream, epoll::Events::EPOLLIN, data)
            .unwrap();
    }
}
