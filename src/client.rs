use std::io::prelude::*;
use std::io::IoSlice;
use std::mem;
use std::net::TcpStream;
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;

const BUFFER_SIZE: usize = 512;

#[derive(Clone)]
pub struct Message {
    user: Rc<String>,
    message: Rc<VectoredData>,
    written: usize,
}

impl Message {
    pub fn write<T: Write>(&mut self, stream: &mut T) -> bool {
        let msg_written = if self.written > self.user.len() {
            self.written - self.user.len()
        } else {
            0
        };

        let buf_index = msg_written / BUFFER_SIZE;
        let mut bufs =
            Vec::with_capacity(1 + self.message.data.len() - buf_index);
        if self.written < self.user.len() {
            bufs.push(IoSlice::new(&self.user.as_bytes()[self.written..]));
        }

        for (index, buffer) in
            self.message.data.iter().skip(buf_index).enumerate()
        {
            let buf_write_index = msg_written % BUFFER_SIZE;
            let buf_end = if index == self.message.data.len() - 1 {
                self.message.data_length % BUFFER_SIZE
            } else {
                BUFFER_SIZE
            };
            let slice = &buffer[buf_write_index..buf_end];
            bufs.push(IoSlice::new(slice));
        }

        let nb_write = stream.write_vectored(bufs.as_slice()).unwrap();

        self.written += nb_write;
        self.written == self.message.data_length + self.user.len()
    }
}

struct VectoredData {
    data: Vec<Rc<[u8; BUFFER_SIZE]>>,
    data_length: usize,
}

impl VectoredData {
    pub fn new() -> VectoredData {
        VectoredData {
            data: Vec::new(),
            data_length: 0,
        }
    }
    pub fn read<T: Read>(&mut self, stream: &mut T) -> usize {
        if self.data.len() * BUFFER_SIZE == self.data_length {
            self.data.push(Rc::new([0; BUFFER_SIZE]));
        }
        let buf_index = self.data_length / BUFFER_SIZE;
        let buf_read_index = self.data_length % BUFFER_SIZE;
        let mut buffer = &mut Rc::get_mut(&mut self.data[buf_index]).unwrap()
            [buf_read_index..BUFFER_SIZE];
        let nb_read = stream.read(&mut buffer).unwrap();
        self.data_length += nb_read;
        nb_read
    }

    pub fn contain(&self, c: u8) -> bool {
        self.data
            .iter()
            .rev()
            .any(|buffer| buffer.iter().any(|&x| x == c))
    }
}

pub struct Client {
    pub stream: TcpStream,
    input: VectoredData,
    output: Vec<Message>,
    user: String,
}

impl Client {
    pub fn new(stream: TcpStream) -> Client {
        Client {
            stream,
            input: VectoredData::new(),
            output: Vec::new(),
            user: String::from("Test:"),
        }
    }
    pub fn read(&mut self) -> usize {
        self.input.read(&mut self.stream)
    }

    pub fn have_message(&self) -> bool {
        self.input.contain(b'\n')
    }

    //pre(self.have_msg())
    pub fn flush_input(&mut self) -> Message {
        let res = mem::replace(&mut self.input, VectoredData::new());
        // TODO: res can contains the beginning of the second message
        Message {
            user: Rc::new(self.user.clone()),
            message: Rc::new(res),
            written: 0,
        }
    }

    pub fn send_message(&mut self, message: &Message) {
        self.output.push(message.clone())
    }

    pub fn write(&mut self) -> bool {
        if self.output[0].write(&mut self.stream) {
            self.output.pop();
            self.output.is_empty()
        } else {
            false
        }
    }
}
