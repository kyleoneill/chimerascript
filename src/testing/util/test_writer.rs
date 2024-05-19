use std::io;
use std::io::Write;

pub struct TestWriter {
    pub buffer: String,
}

impl TestWriter {
    #[allow(dead_code)] // Used in test
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
    #[allow(dead_code)] // Used in test
    pub fn str_lines(&self) -> Vec<&str> {
        self.buffer.lines().collect::<Vec<&str>>()
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.buffer.push_str(s);
                Ok(buf.len())
            }
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 sequence",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // Empty buffer and then Ok?
        Ok(())
    }
}
