//! Fixed-size scrollback held in memory.
//!
//! Overwrites the oldest bytes rather than growing. A terminal left running a
//! verbose build produces output without limit, and a buffer that grows to
//! match is a leak that only shows up after hours.

pub struct RingBuffer {
    buf: Vec<u8>,
    /// Where the next write lands.
    head: usize,
    /// True once the buffer has wrapped at least once.
    wrapped: bool,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0; capacity],
            head: 0,
            wrapped: false,
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let cap = self.buf.len();
        if cap == 0 {
            return;
        }

        // A write larger than the whole buffer can only leave its tail; copying
        // the earlier part would just be overwritten by the rest of the same
        // call.
        let data = if data.len() > cap {
            self.wrapped = true;
            &data[data.len() - cap..]
        } else {
            data
        };

        let until_end = cap - self.head;
        if data.len() <= until_end {
            self.buf[self.head..self.head + data.len()].copy_from_slice(data);
            self.head += data.len();
            if self.head == cap {
                self.head = 0;
                self.wrapped = true;
            }
        } else {
            self.buf[self.head..].copy_from_slice(&data[..until_end]);
            let rest = &data[until_end..];
            self.buf[..rest.len()].copy_from_slice(rest);
            self.head = rest.len();
            self.wrapped = true;
        }
    }

    /// The contents, oldest byte first.
    pub fn contents(&self) -> Vec<u8> {
        if !self.wrapped {
            return self.buf[..self.head].to_vec();
        }
        let mut out = Vec::with_capacity(self.buf.len());
        out.extend_from_slice(&self.buf[self.head..]);
        out.extend_from_slice(&self.buf[..self.head]);
        out
    }

    pub fn len(&self) -> usize {
        if self.wrapped {
            self.buf.len()
        } else {
            self.head
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_everything_while_it_fits() {
        let mut ring = RingBuffer::new(16);
        ring.write(b"quockpit");
        assert_eq!(ring.contents(), b"quockpit");
        assert_eq!(ring.len(), 8);
    }

    #[test]
    fn drops_the_oldest_once_it_wraps() {
        let mut ring = RingBuffer::new(8);
        ring.write(b"abcdefgh");
        ring.write(b"XY");
        // The two oldest bytes are gone, and the order of the rest holds.
        assert_eq!(ring.contents(), b"cdefghXY");
        assert_eq!(ring.len(), 8);
    }

    #[test]
    fn a_write_larger_than_the_buffer_leaves_its_tail() {
        let mut ring = RingBuffer::new(4);
        ring.write(b"abcdefghij");
        assert_eq!(ring.contents(), b"ghij");
    }

    #[test]
    fn a_write_that_lands_exactly_on_the_end_does_not_lose_a_byte() {
        let mut ring = RingBuffer::new(4);
        ring.write(b"abcd");
        assert_eq!(ring.contents(), b"abcd");
        ring.write(b"e");
        assert_eq!(ring.contents(), b"bcde");
    }
}
