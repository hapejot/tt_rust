use std::cmp::min;

use bytebuffer::ByteBuffer;

use super::{ReadingContent, INode};

pub struct BytesContent{
    ino: u64,
    buf:ByteBuffer,
}

impl BytesContent {
    pub fn new_from_str(ino: u64, val: &str) -> Self {
        let mut bb = ByteBuffer::new();
        bb.write_string(val);
        BytesContent{ ino, buf: bb }
    }
}

impl ReadingContent for BytesContent {
    fn inode(&self) -> INode {
        INode{ id: self.ino, kind: super::NodeType::File, size: self.buf.len() }
    }

    fn read(&mut self, offset: i64, size: u32) -> ByteBuffer {
        let buf = &self.buf;
        let from = offset as usize;
        let to = min(offset as usize + size as usize, buf.len());
        ByteBuffer::from_bytes(&buf.as_bytes()[from..to])
    }
}
