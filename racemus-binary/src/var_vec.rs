#[derive(Debug, Copy, Clone)]
struct VarVecLayout {
    bits_per_entry: usize,
    capacity: usize,
    value_mask: u64,
}

impl VarVecLayout {
    #[inline]
    fn new(capacity: usize, bits_per_entry: u8) -> Self {
        assert!(bits_per_entry <= 64);
        let bits_per_entry = bits_per_entry as usize;

        VarVecLayout {
            bits_per_entry,
            capacity,
            value_mask: std::u64::MAX >> (64 - bits_per_entry),
        }
    }

    #[inline]
    fn required_entries(&self) -> usize {
        let total_bits = self.capacity.checked_mul(self.bits_per_entry).unwrap();
        let whole_entries = (total_bits + 64 - 1) / 64;
        whole_entries
    }

    #[inline]
    fn calculate_offsets(&self, index: usize) -> (usize, usize, usize) {
        let bit_index = index * self.bits_per_entry;
        let entry_index = bit_index / 64;
        let bit_offset = bit_index % 64;
        let end_bit_offset = bit_offset + self.bits_per_entry;
        (entry_index, bit_offset, end_bit_offset)
    }

    #[inline]
    fn get(&self, entries: &[u64], index: usize) -> Option<u64> {
        if index >= self.capacity {
            return None;
        }

        let (entry_index, bit_offset, end_bit_offset) = self.calculate_offsets(index);

        let mut value = entries.get(entry_index).unwrap() >> bit_offset;

        if end_bit_offset > 64 {
            value |= entries.get(entry_index + 1).unwrap() << (64 - bit_offset);
        }

        Some(value & self.value_mask)
    }

    #[inline]
    fn set(&self, entries: &mut [u64], index: usize, value: u64) -> Option<u64> {
        if index >= self.capacity {
            return None;
        }

        let (entry_index, bit_offset, end_bit_offset) = self.calculate_offsets(index);

        let mut old;
        let entry = entries.get_mut(entry_index).unwrap();

        old = *entry >> bit_offset;
        *entry &= !(self.value_mask << bit_offset); // Clear bits
        *entry |= (self.value_mask & value) << bit_offset; // Set bits

        if end_bit_offset > 64 {
            let entry = entries.get_mut(entry_index + 1).unwrap();

            old |= *entry << (64 - bit_offset);
            *entry &= std::u64::MAX << (end_bit_offset - 64); // Clear bits
            *entry |= (self.value_mask & value) >> (64 - bit_offset); // Set bits
        }

        Some(old & self.value_mask)
    }
}

#[derive(Debug)]
pub struct VarVec {
    entries: Vec<u64>,
    layout: VarVecLayout,
}

pub fn ceil_log2(n: u64) -> u8 {
    (64 - n.leading_zeros() as u64 - n.is_power_of_two() as u64) as u8
}

impl VarVec {
    #[inline]
    pub fn new(bits_per_entry: u8) -> Self {
        Self::with_capacity(0, bits_per_entry)
    }

    #[inline]
    pub fn with_capacity(capacity: usize, bits_per_entry: u8) -> Self {
        let layout = VarVecLayout::new(capacity, bits_per_entry);

        let mut entries = Vec::new();
        entries.resize(layout.required_entries(), 0);

        Self { entries, layout }
    }

    #[inline]
    pub fn get_inner(&self) -> &[u64] {
        &self.entries
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<u64> {
        self.layout.get(&self.entries, index)
    }

    #[inline]
    pub fn set(&mut self, index: usize, value: u64) -> Option<u64> {
        self.layout.set(&mut self.entries, index, value)
    }

    #[inline]
    pub fn set_slice(&mut self, index: usize, value: &[u64]) {
        for i in 0..value.len() {
            self.layout.set(&mut self.entries, index + i, value[i]);
        }
    }

    pub fn resize(&mut self, capacity: usize) {
        if self.layout.capacity == capacity {
            return;
        };

        self.layout = VarVecLayout::new(capacity, self.layout.bits_per_entry as u8);
        self.entries.resize(self.layout.required_entries(), 0);
    }

    pub fn resize_bits_per_entry(&mut self, bits_per_entry: u8) {
        if self.layout.bits_per_entry == bits_per_entry as usize {
            return;
        }

        let capacity = self.layout.capacity;
        let old = std::mem::replace(
            &mut self.layout,
            VarVecLayout::new(capacity, bits_per_entry),
        );

        if self.layout.bits_per_entry < old.bits_per_entry {
            let mut index = 0;
            loop {
                match old.get(&self.entries, index) {
                    Some(value) => self.layout.set(&mut self.entries, index, value),
                    None => break,
                };
                index += 1;
            }
            self.entries.resize(self.layout.required_entries(), 0);
        } else {
            let mut index = capacity;
            self.entries.resize(self.layout.required_entries(), 0);
            while index > 0 {
                index -= 1;
                match old.get(&self.entries, index) {
                    Some(value) => {
                        self.layout.set(&mut self.entries, index, value);
                    }
                    None => continue,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn bit_writer_vector_1() {
        // Test vector from: https://wiki.vg/Chunk_Format#Example
        // First example
        // Note that the example incorrectly omits 2 bits of the final '4'

        let mut b = VarVec::with_capacity(26, 5);
        b.set_slice(
            0,
            &[
                1, 2, 2, 3, 4, 4, 5, 6, 6, 4, 8, 0, 7, 4, 3, 13, 15, 16, 9, 14, 10, 12, 0, 2, 11, 4,
            ][..],
        );
        assert_eq!(
            &[0x7020863148418841, 0x8b1018a7260f68c8, 0x0][..],
            b.get_inner()
        );
    }

    #[test]
    pub fn bit_writer_vector_2() {
        // Test vector from: https://wiki.vg/Chunk_Format#Example
        // Second example
        // Note that the example incorrectly omits 1 bits of the final '4'

        let mut b = VarVec::with_capacity(10, 13);
        b.set_slice(
            0,
            &[0x20, 0x30, 0x30, 0x31, 0x10, 0x10, 0x13, 0xd0, 0xd0, 0x810][..],
        );
        assert_eq!(
            &[0x01001880c0060020, 0x0200d0068004c020, 0x1][..],
            b.get_inner()
        );
    }

    #[test]
    pub fn bit_writer_set() {
        let mut b = VarVec::with_capacity(16, 5);

        for i in 0..16 {
            for j in 0..16 {
                b.set(i, j);
                assert_eq!((i, Some(j)), (i, b.get(i)));
            }
            for j in 16..=0 {
                b.set(i, j);
                assert_eq!((i, Some(j)), (i, b.get(i)));
            }
        }
    }

    #[test]
    pub fn ceil_log2_test() {
        assert_eq!(13, ceil_log2(8192));
        assert_eq!(14, ceil_log2(8193));
        assert_eq!(14, ceil_log2(16384));
        assert_eq!(16, ceil_log2(65535));
        assert_eq!(16, ceil_log2(65536));
        assert_eq!(17, ceil_log2(65537));
    }

    #[test]
    pub fn bit_writer_resize_bits_per_entry() {
        let values = [
            1u64, 2, 2, 3, 4, 4, 5, 6, 6, 4, 8, 0, 7, 4, 3, 13, 15, 7, 9, 14, 10, 12, 0, 2, 11, 4,
        ];

        let mut b = VarVec::with_capacity(26, 5);

        b.set_slice(0, &values[..]);
        for i in 0..values.len() {
            assert_eq!(Some(values[i]), b.get(i));
        }

        b.resize_bits_per_entry(4);
        for i in 0..values.len() {
            assert_eq!(Some(values[i]), b.get(i));
        }

        b.resize_bits_per_entry(6);
        b.resize_bits_per_entry(6);
        for i in 0..values.len() {
            assert_eq!(Some(values[i]), b.get(i));
        }
    }

    #[test]
    pub fn bit_writer_resize() {
        let values = [
            1u64, 2, 2, 3, 4, 4, 5, 6, 6, 4, 8, 0, 7, 4, 3, 13, 15, 7, 9, 14, 10, 12, 0, 2, 11, 4,
        ];

        let mut b = VarVec::with_capacity(26, 5);

        b.set_slice(0, &values[..]);
        b.resize(5);
        b.resize(5);
        for i in 0..5 {
            assert_eq!(Some(values[i]), b.get(i));
        }

        assert_eq!(None, b.get(5));
        assert_eq!(None, b.set(5, 1));
    }
}
