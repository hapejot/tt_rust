use std::cmp::min;

use bytebuffer::ByteBuffer;

use super::ReadingContent;

pub struct BytesContent(ByteBuffer);

impl BytesContent {
    pub fn new_from_str(val: &str) -> Self {
        let mut bb = ByteBuffer::new();
        bb.write_string(val);
        BytesContent(bb)
    }
}

impl ReadingContent for BytesContent {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read(&mut self, offset: i64, size: u32) -> ByteBuffer {
        let buf = &self.0;
        let from = offset as usize;
        let to = min(offset as usize + size as usize, buf.len());
        ByteBuffer::from_bytes(&buf.as_bytes()[from..to])
    }
}
