// based on https://github.com/sozu-proxy/circular
//

use std::io::{self, Read, Write};
use std::iter::repeat;
use std::{cmp, ptr};

/// the Buffer contains the underlying memory and data positions
///
/// In all cases, `0 ≤ position ≤ end ≤ capacity` should be true
#[derive(Debug, PartialEq, Clone)]
pub struct Buffer {
    /// the Vec containing the data
    memory: Vec<u8>,
    /// the current capacity of the Buffer
    capacity: usize,
    /// the current beginning of the available data
    position: usize,
    /// the current end of the available data
    /// and beginning of the available space
    end: usize,
}

impl Buffer {
    /// allocates a new buffer of maximum size `capacity`
    pub fn with_capacity(capacity: usize) -> Buffer {
        let mut v = Vec::with_capacity(capacity);
        v.extend(repeat(0).take(capacity));
        Buffer {
            memory: v,
            capacity: capacity,
            position: 0,
            end: 0,
        }
    }

    /// increases the size of the buffer
    ///
    /// this does nothing if the buffer is already large enough
    pub fn grow(&mut self, new_size: usize) -> bool {
        if self.capacity >= new_size {
            return false;
        }

        self.memory.resize(new_size, 0);
        self.capacity = new_size;
        true
    }

    /// returns how much data can be read from the buffer
    pub fn available_data(&self) -> usize {
        self.end - self.position
    }

    /// returns how much free space is available to write to
    pub fn available_space(&self) -> usize {
        self.capacity - self.end
    }

    /// advances the position tracker
    ///
    /// if the position gets past the buffer's half,
    /// this will call `shift()` to move the remaining data
    /// to the beginning of the buffer
    pub fn consume(&mut self, count: usize) -> usize {
        let cnt = cmp::min(count, self.available_data());
        self.position += cnt;
        if self.position > self.capacity / 2 {
            //trace!("consume shift: pos {}, end {}", self.position, self.end);
            self.shift();
        }
        cnt
    }

    /// advances the position tracker
    ///
    /// This method is similar to `consume()` but will not move data
    /// to the beginning of the buffer
    pub fn consume_noshift(&mut self, count: usize) -> usize {
        let cnt = cmp::min(count, self.available_data());
        self.position += cnt;
        cnt
    }

    /// after having written data to the buffer, use this function
    /// to indicate how many bytes were written
    ///
    /// if there is not enough available space, this function can call
    /// `shift()` to move the remaining data to the beginning of the
    /// buffer
    pub fn fill(&mut self, count: usize) -> usize {
        let cnt = cmp::min(count, self.available_space());
        self.end += cnt;
        if self.available_space() < self.available_data() + cnt {
            //trace!("fill shift: pos {}, end {}", self.position, self.end);
            self.shift();
        }

        cnt
    }

    /// Get the current position
    ///
    /// # Examples
    /// ```
    /// use circular::Buffer;
    /// use std::io::{Read,Write};
    ///
    /// let mut output = [0;5];
    ///
    /// let mut b = Buffer::with_capacity(10);
    ///
    /// let res = b.write(&b"abcdefgh"[..]);
    ///
    /// b.read(&mut output);
    ///
    /// // Position must be 5
    /// assert_eq!(b.position(), 5);
    /// assert_eq!(b.available_data(), 3);
    /// ```
    #[cfg(test)]
    pub fn position(&self) -> usize {
        self.position
    }

    /// returns a slice with all the available data
    pub fn data(&self) -> &[u8] {
        &self.memory[self.position..self.end]
    }

    /// returns a mutable slice with all the available space to
    /// write to
    pub fn space(&mut self) -> &mut [u8] {
        &mut self.memory[self.end..self.capacity]
    }

    /// moves the data at the beginning of the buffer
    ///
    /// if the position was more than 0, it is now 0
    pub fn shift(&mut self) {
        if self.position > 0 {
            unsafe {
                let length = self.end - self.position;
                ptr::copy(
                    (&self.memory[self.position..self.end]).as_ptr(),
                    (&mut self.memory[..length]).as_mut_ptr(),
                    length,
                );
                self.position = 0;
                self.end = length;
            }
        }
    }

    pub fn replace_slice(&mut self, range: std::ops::Range<usize>, data: &[u8]) -> Option<usize> {
        let data_len = data.len();
        if range.end > self.available_data()
            || self.position + range.start + data_len > self.capacity
        {
            return None;
        }

        let start = self.position + range.start;
        let remove_range = start..(self.position + range.end);
        let add_range = start..(start + data.len());
        self.memory
            .splice(remove_range.clone(), data.iter().cloned());
        self.memory.resize(self.capacity, 0);

        if add_range.len() > remove_range.len() {
            self.end += add_range.len() - remove_range.len();
        } else {
            self.end -= remove_range.len() - add_range.len();
        }

        Some(self.available_data())
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.space().write(buf) {
            Ok(size) => {
                self.fill(size);
                Ok(size)
            }
            err => err,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(self.available_data(), buf.len());
        unsafe {
            ptr::copy(
                (&self.memory[self.position..self.position + len]).as_ptr(),
                buf.as_mut_ptr(),
                len,
            );
            self.position += len;
        }
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn fill_and_consume() {
        let mut b = Buffer::with_capacity(10);
        assert_eq!(b.available_data(), 0);
        assert_eq!(b.available_space(), 10);
        let res = b.write(&b"abcd"[..]);
        assert_eq!(res.ok(), Some(4));
        assert_eq!(b.available_data(), 4);
        assert_eq!(b.available_space(), 6);

        assert_eq!(b.data(), &b"abcd"[..]);

        b.consume(2);
        assert_eq!(b.available_data(), 2);
        assert_eq!(b.available_space(), 6);
        assert_eq!(b.data(), &b"cd"[..]);

        b.shift();
        assert_eq!(b.available_data(), 2);
        assert_eq!(b.available_space(), 8);
        assert_eq!(b.data(), &b"cd"[..]);

        assert_eq!(b.write(&b"efghijklmnop"[..]).ok(), Some(8));
        assert_eq!(b.available_data(), 10);
        assert_eq!(b.available_space(), 0);
        assert_eq!(b.data(), &b"cdefghijkl"[..]);
        b.shift();
        assert_eq!(b.available_data(), 10);
        assert_eq!(b.available_space(), 0);
        assert_eq!(b.data(), &b"cdefghijkl"[..]);
    }

    #[test]
    fn replace() {
        let mut b = Buffer::with_capacity(10);
        let _ = b.write(&b"abcdefgh"[..]);
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);

        assert_eq!(b.replace_slice(2..5, &b"ABC"[..]), Some(8));
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);
        assert_eq!(b.data(), &b"abABCfgh"[..]);

        assert_eq!(b.replace_slice(8..11, &b"XYZ"[..]), None);
        assert_eq!(b.replace_slice(6..9, &b"XYZ"[..]), None);

        assert_eq!(b.replace_slice(2..6, &b"XYZ"[..]), Some(7));
        assert_eq!(b.available_data(), 7);
        assert_eq!(b.available_space(), 3);
        assert_eq!(b.data(), &b"abXYZgh"[..]);

        assert_eq!(b.replace_slice(2..4, &b"123"[..]), Some(8));
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);
        assert_eq!(b.data(), &b"ab123Zgh"[..]);
    }

    #[test]
    fn consume_without_shift() {
        let mut b = Buffer::with_capacity(10);
        let _ = b.write(&b"abcdefgh"[..]);
        b.consume_noshift(6);
        assert_eq!(b.position(), 6);
    }
}
