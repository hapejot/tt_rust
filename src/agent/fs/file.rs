use std::{
    fs::File,
    io::{Read, Write},
};

use bytebuffer::ByteBuffer;
use tracing::info;

use super::{Content, ReadingContent, WritingContent};

pub struct FileContent {
    file_name: String,
    file: File,
}

impl FileContent {
    pub fn new(file_name: String) -> Self {
        Self {
            file_name: file_name.clone(),
            file: File::options()
                .read(true)
                .write(true)
                .open(file_name)
                .unwrap(),
        }
    }
}

impl WritingContent for FileContent {
    fn write(&mut self, offset: i64, size: u32, data: ByteBuffer) -> u32 {
        self.file.write(data.as_bytes()).unwrap() as u32
    }

    fn flush(&mut self) {
        info!("flush File Content");
    }
}

impl ReadingContent for FileContent {
    fn len(&self) -> u64 {
        let m = self.file.metadata().unwrap();
        m.len()
    }

    fn read(&mut self, offset: i64, size: u32) -> ByteBuffer {
        let mut buf = ByteBuffer::new();
        let mut b = Vec::new();
        b.resize(size as usize, 0u8);
        let n = self.file.read(&mut b[..]).unwrap();
        info!("read {n} bytes from file.");
        buf.write_bytes(&b[..n]);
        buf
    }
}

impl Content for FileContent {}
