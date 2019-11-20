use std::cell::RefCell;
use std::io::prelude::*;
use std::mem;
use std::net::TcpStream;
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;

const BUFFER_SIZE: usize = 512;

#[derive(Clone)]
pub struct Message {
    user: String,
    message: Rc<VectoredData>,
    written: usize,
}

impl Message {
    pub fn write<T: Write>(&mut self, stream: &mut T) -> bool {
        let mut msg_written = self.written;
        let buf_index = msg_written / BUFFER_SIZE;
        let buf_write_index = msg_written % BUFFER_SIZE;
        let buf_end = if buf_index == self.message.data.len() - 1 {
            self.message.data_length % BUFFER_SIZE
        } else {
            BUFFER_SIZE
        };
        let buffer =
            &self.message.data[buf_index].borrow()[buf_write_index..buf_end];
        let nb_write = stream.write(&buffer).unwrap();
        msg_written += nb_write;
        self.written += nb_write;
        msg_written == self.message.data_length
    }
}

struct VectoredData {
    data: Vec<Rc<RefCell<[u8; BUFFER_SIZE]>>>,
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
            self.data.push(Rc::new(RefCell::new([0; BUFFER_SIZE])));
        }
        let buf_index = self.data_length / BUFFER_SIZE;
        let buf_read_index = self.data_length % BUFFER_SIZE;
        let buffer =
            &mut self.data[buf_index].borrow_mut()[buf_read_index..BUFFER_SIZE];
        let nb_read = stream.read(buffer).unwrap();
        self.data_length += nb_read;
        nb_read
    }

    pub fn contain(&self, c: u8) -> bool {
        self.data
            .iter()
            .rev()
            .any(|buffer| buffer.borrow().iter().any(|&x| x == c))
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
        self.input.contain('\n' as u8)
    }

    //pre(self.have_msg())
    pub fn flush_input(&mut self) -> Message {
        let res = mem::replace(&mut self.input, VectoredData::new());
        // TODO: res can contains the beginning of the second message
        Message {
            user: self.user.clone(),
            message: Rc::new(res),
            written: 0,
        }
    }

    pub fn send_message(&mut self, message: &Message) {
        self.output.push(message.clone())
    }

    pub fn write(&mut self) -> bool {
        //TODO: handle username when iovec will be used
        if self.output[0].write(&mut self.stream) {
            self.output.pop();
            self.output.is_empty()
        } else {
            false
        }
    }
}
