#![feature(ptr_internals)]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]

use std::{
    alloc::{handle_alloc_error, AllocRef, Global, Layout, AllocInit, ReallocPlacement},
    cmp,
    convert::TryInto,
    io::{self, Write},
    mem,
    ops::{Deref, DerefMut, Range},
    ptr::{self, NonNull, Unique},
};

#[derive(Debug, Clone)]
struct RawBuf<A: AllocRef = Global> {
    ptr: Unique<u8>,
    increment: usize,
    capacity: usize,
    a: A,
}

const ELEM_SIZE: usize = mem::size_of::<u8>();
const ALIGN: usize = mem::align_of::<u8>();

impl<A: AllocRef> RawBuf<A> {
    fn new(a: A, increment: usize) -> Self {
        Self {
            ptr: Unique::empty(),
            increment,
            capacity: 0,
            a,
        }
    }

    fn ensure(&mut self, desired: usize) -> bool {
        if desired <= self.capacity {
            return false;
        }

        fn capacity_overflow() -> ! {
            panic!("capacity overflow");
        }

        // Size it to `increment` increments
        let new_capacity = (desired + self.increment - 1) / self.increment;
        let new_capacity = new_capacity * self.increment;
        
        let new_layout = Layout::array::<u8>(new_capacity).unwrap_or_else(|_| capacity_overflow());
        if mem::size_of::<usize>() < 8 && new_layout.size() > isize::MAX as usize {
            capacity_overflow()
        }

        let ptr = if self.capacity == 0 {
            self.a.alloc(new_layout, AllocInit::Uninitialized)
        } else {
            let c: NonNull<u8> = self.ptr.into();
            unsafe {
                let old_layout = Layout::from_size_align_unchecked(ELEM_SIZE * self.capacity, ALIGN);
                self.a.grow(
                    c,
                    old_layout,
                    new_layout.size(),
                    ReallocPlacement::MayMove,
                    AllocInit::Uninitialized
                )
            }
        };

        if let Ok(memory) = ptr {
            self.ptr = memory.ptr.cast().into();
            self.capacity = memory.size;
        } else {
            handle_alloc_error(new_layout);
        }

        true
    }

    fn set(&mut self, index: usize, data: &[u8]) {
        assert!(index + data.len() <= self.capacity);
        let index = index.try_into().unwrap();
        let len = data.len();
        unsafe {
            ptr::copy(data.as_ptr(), self.ptr.as_ptr().offset(index), len);
        }
    }

    fn shift(&mut self, range: Range<usize>) {
        let len = range.len();
        assert!(len <= self.capacity);
        let start = range.start.try_into().unwrap();

        unsafe {
            ptr::copy(self.ptr.as_ptr().offset(start), self.ptr.as_ptr(), len);
        }
    }

    fn remove_insert(&mut self, remove: Range<usize>, insert: usize, valid: usize) {
        assert!(valid <= self.capacity);
        assert!(remove.end <= valid);

        if insert > remove.len() {
            assert!(remove.start + insert - remove.len() <= self.capacity);
        }

        let src_start = remove.end.try_into().unwrap();
        let dst_start = (remove.start + insert).try_into().unwrap();
        let count = valid - remove.end;

        unsafe {
            ptr::copy(
                self.ptr.as_ptr().offset(src_start),
                self.ptr.as_ptr().offset(dst_start),
                count,
            );
        }
    }
}

impl<A: AllocRef> Deref for RawBuf<A> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { ::std::slice::from_raw_parts(self.ptr.as_ptr(), self.capacity) }
    }
}

impl<A: AllocRef> DerefMut for RawBuf<A> {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { ::std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.capacity) }
    }
}

impl<A: AllocRef> Drop for RawBuf<A> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            unsafe {
                let c: NonNull<u8> = self.ptr.into();
                Global.dealloc(c.cast(), Layout::array::<u8>(self.capacity).unwrap());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Buffer<A: AllocRef = Global> {
    memory: RawBuf<A>,
    current: Range<usize>,
}

impl Buffer<Global> {
    pub fn with_capacity(increment: usize, capacity: usize) -> Self {
        let mut memory = RawBuf::new(Global, increment);
        memory.ensure(capacity);
        Buffer {
            memory,
            current: 0..0,
        }
    }
}

impl<A: AllocRef> Buffer<A> {
    pub fn with_capacity_in(a: A, increment: usize, capacity: usize) -> Self {
        let mut memory = RawBuf::new(a, increment);
        memory.ensure(capacity);
        Buffer {
            memory,
            current: 0..0,
        }
    }

    pub fn ensure_space(&mut self, capacity: usize) -> bool {
        if capacity > self.available_space() {
            self.shift();
            if capacity > self.available_space() {
                self.memory.ensure(self.available_data() + capacity);
                return true;
            }
        }
        false
    }

    pub fn available_data(&self) -> usize {
        self.current.len()
    }

    pub fn available_space(&self) -> usize {
        self.memory.capacity - self.current.end
    }

    pub fn consume(&mut self, count: usize) -> usize {
        let cnt = cmp::min(count, self.available_data());
        self.current = (self.current.start + cnt)..self.current.end;
        cnt
    }

    pub fn fill(&mut self, count: usize) -> usize {
        let cnt = cmp::min(count, self.available_space());
        self.current = self.current.start..(self.current.end + cnt);
        cnt
    }

    pub fn clear(&mut self) {
        self.current = 0..0;
    }

    pub fn position(&self) -> usize {
        self.current.start
    }

    pub fn data(&self) -> &[u8] {
        &self.memory[self.current.clone()]
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.memory[self.current.clone()]
    }

    pub fn space(&mut self) -> &mut [u8] {
        let capacity = self.memory.capacity;
        &mut self.memory[self.current.end..capacity]
    }

    pub fn append(&mut self, data: &[u8]) -> Option<usize> {
        if self.current.end + data.len() > self.memory.capacity {
            return None;
        }

        self.memory.set(self.current.end, data);
        self.current = self.current.start..(self.current.end + data.len());
        Some(self.available_data())
    }

    pub fn shift(&mut self) {
        if self.current.start > 0 {
            let len = self.current.len();
            let old = mem::replace(&mut self.current, 0..len);
            if len != 0 {
                self.memory.shift(old);
            }
        }
    }

    pub fn replace_slice(&mut self, range: Range<usize>, data: &[u8]) -> Option<usize> {
        let data_len = data.len();
        let start = range.start;
        let remove_len = range.len();

        if range.end > self.available_data()
            || self.current.start + start + data_len > self.memory.capacity
        {
            return None;
        }

        self.memory
            .remove_insert(range, data.len(), self.current.end);
        self.memory.set(start, data);

        if data_len > remove_len {
            self.current = self.current.start..(self.current.end + data_len - remove_len);
        } else {
            self.current = self.current.start..(self.current.end - (remove_len - data_len));
        }

        Some(self.available_data())
    }
}

impl<A: AllocRef> Write for Buffer<A> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn append(buf: &mut Buffer, data: &[u8]) {
        let len = cmp::min(buf.available_space(), data.len());
        (&mut buf.space()[0..len]).copy_from_slice(&data[0..len]);
        buf.fill(len);
    }

    #[test]
    fn fill_and_consume() {
        let mut b = Buffer::with_capacity(1, 10);
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
    fn consume_without_shift() {
        let mut b = Buffer::with_capacity(1, 10);
        append(&mut b, &b"abcdefgh"[..]);
        b.consume(6);
        assert_eq!(b.position(), 6);
    }

    #[test]
    fn replace() {
        let mut b = Buffer::with_capacity(1, 10);
        append(&mut b, &b"abcdefgh"[..]);
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
}
