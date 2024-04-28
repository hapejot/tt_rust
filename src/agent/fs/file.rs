use std::{
    fs::File,
    io::Write, os::unix::fs::FileExt,
};

use bytebuffer::ByteBuffer;
use tracing::info;

use super::{Content, INode, ReadingContent, WritingContent};

pub struct FileContent {
    file_name: String,
    file: File,
    ino: u64,
}

impl FileContent {
    pub fn new(ino: u64, file_name: String) -> Self {
        Self {
            ino,
            file_name: file_name.clone(),
            file: File::options()
                .read(true)
                .write(true)
                .create(true)
                .open(file_name)
                .unwrap(),
        }
    }
}

impl WritingContent for FileContent {
    fn write(&mut self, _offset: i64, _size: u32, data: ByteBuffer) -> u32 {
        info!("write {}", self.file_name);
        self.file.write(data.as_bytes()).unwrap() as u32
    }

    fn flush(&mut self) {
        info!("flush File Content");
    }
}

impl ReadingContent for FileContent {
    fn inode(&self) -> INode {
        let m = self.file.metadata().unwrap();
        INode {
            id: self.ino,
            kind: super::NodeType::File,
            size: m.len() as usize,
        }
    }

    fn read(&mut self, _offset: i64, size: u32) -> ByteBuffer {
        let mut buf = ByteBuffer::new();
        let mut b = Vec::new();
        b.resize(size as usize, 0u8);
        let n = self.file.read_at(&mut b[..],_offset as u64).unwrap();
        info!("read {n} bytes from file.");
        buf.write_bytes(&b[..n]);
        buf
    }
}

impl Content for FileContent {}
