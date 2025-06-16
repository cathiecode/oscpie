use std::io::{Read, Write};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use serde_json::{Result, Error};

pub struct Sender<T, W>
where
    T: Serialize,
    W: Write,
{
    to: W,
    phantom: std::marker::PhantomData<T>,
}

impl<T, W> Sender<T, W>
where
    T: Serialize,
    W: Write,
{
    pub fn send<'a>(&mut self, data: T) -> Result<()>
    where
        T: Serialize + Deserialize<'a>,
    {
        serde_json::to_writer(&mut self.to, &data)?;
        self.to.write_all(b"\n").map_err(Error::io)?;
        self.to.flush().map_err(Error::io)?;
        Ok(())
    }
}

pub fn sender<T, W>(to: W) -> Sender<T, W>
where
    T: Serialize,
    W: Write,
{
    Sender {
        to,
        phantom: std::marker::PhantomData,
    }
}

struct ReadUntilNewline<R>
where
    R: std::io::Read,
{
    reader: R,
}

impl<R> ReadUntilNewline<R>
where
    R: std::io::Read,
{
    pub fn new(reader: R) -> Self {
        ReadUntilNewline { reader }
    }
}

impl<R> Read for ReadUntilNewline<R>
where
    R: std::io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut total_read = 0;
        loop {
            let mut byte = [0; 1];
            let bytes_read = self.reader.read(&mut byte)?;
            if bytes_read == 0 || byte[0] == b'\n' {
                break;
            }
            if total_read < buf.len() {
                buf[total_read] = byte[0];
                total_read += 1;
            }

            if total_read == buf.len() {
                break;
            }
        }
        Ok(total_read)
    }
}

pub struct Receiver<T, R>
where
    T: DeserializeOwned,
    R: std::io::Read,
{
    from: R,
    phantom: std::marker::PhantomData<T>,
}

impl<T, R> Receiver<T, R>
where
    T: DeserializeOwned,
    R: std::io::Read,
{
    pub fn recv(&mut self) -> Result<T> {
        serde_json::from_reader(ReadUntilNewline::new(&mut self.from))
    }
}

pub fn receiver<T, R>(from: R) -> Receiver<T, R>
where
    T: DeserializeOwned,
    R: std::io::Read,
{
    Receiver {
        from,
        phantom: std::marker::PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        collections::VecDeque,
        io::{BufReader, BufWriter},
        rc::Rc,
    };

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct SomeNiceType {
        value: u64,
    }

    #[derive(Clone, Debug)]
    struct MockPipe {
        data: Rc<RefCell<VecDeque<u8>>>,
    }

    impl MockPipe {
        fn new() -> Self {
            MockPipe {
                data: Rc::new(RefCell::new(VecDeque::new())),
            }
        }
    }

    impl Write for MockPipe {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.data.borrow_mut().extend(buf.iter().cloned());
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Read for MockPipe {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.data.borrow().is_empty() {
                return Ok(0);
            }
            let mut bytes_read = 0;
            while bytes_read < buf.len() && !self.data.borrow().is_empty() {
                buf[bytes_read] = self.data.borrow_mut().pop_front().unwrap();
                bytes_read += 1;
            }
            Ok(bytes_read)
        }
    }

    #[test]
    fn send_and_recv_works() {
        let pipe = MockPipe::new();

        let mut sender = sender::<SomeNiceType, _>(BufWriter::new(pipe.clone()));
        let mut receiver = receiver::<SomeNiceType, _>(BufReader::new(pipe.clone()));

        sender.send(SomeNiceType { value: 42 }).unwrap();
        sender.send(SomeNiceType { value: 43 }).unwrap();
        sender.send(SomeNiceType { value: 44 }).unwrap();

        assert_eq!(receiver.recv().unwrap().value, 42);
        assert_eq!(receiver.recv().unwrap().value, 43);
        assert_eq!(receiver.recv().unwrap().value, 44);
    }
}
