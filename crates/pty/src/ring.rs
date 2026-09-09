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
            // Keeps head inside 0..cap. Nothing downstream needs it: at
            // head == cap every write takes the branch below, which lands on
            // the same contents, and both `contents` and `len` answer the
            // same either way — a mutation sweep found this and it took a
            // property test over a thousand writes to be sure. It stays
            // because the invariant is what makes the slicing here obviously
            // in bounds, but it is a normalisation, not a fix.
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

    /// The contents from the first whole character onward.
    ///
    /// The buffer drops bytes by count, so once it has wrapped the oldest byte
    /// is as likely as not to be the middle of a multi-byte character. Handing
    /// that to a terminal draws a replacement character at the top of the
    /// replay, and on a screen full of accented text it draws several.
    ///
    /// The fix is to start one character later, not to teach the buffer about
    /// text: a ring that understood encodings would have to decode on every
    /// write, and this one is on the read path of every byte a pty produces.
    pub fn replay(&self) -> Vec<u8> {
        let mut bytes = self.contents();
        // Continuation bytes are `10xxxxxx`. Anything else starts a character,
        // and the first one of those is where a decoder can begin.
        let start = bytes
            .iter()
            .position(|byte| byte & 0xc0 != 0x80)
            .unwrap_or(bytes.len());
        bytes.drain(..start);
        bytes
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
#[path = "ring_tests.rs"]
mod tests;
