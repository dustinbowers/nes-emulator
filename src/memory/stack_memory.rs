use crate::memory::memory_trait::MemoryTrait;

pub struct StackMemory<T: Sized + Copy, const N: usize> {
    data: [T; N], // Fixed-size array allocated on the stack
}

impl<T: Sized + Copy, const N: usize> MemoryTrait<T> for StackMemory<T, N> {
    fn new(_size: usize, default: T) -> Self {
        Self {
            data: [default; N], // Initialize all elements with the default value
        }
    }

    fn get_size(&self) -> usize {
        N // The size is known at compile time
    }

    fn read(&self, address: usize) -> &T {
        &self.data[address]
    }

    fn read_n(&self, address: usize, n: usize) -> &[T] {
        &self.data[address..address + n]
    }

    fn write(&mut self, address: usize, data: T) {
        self.data[address] = data;
    }

    fn write_n(&mut self, address: usize, data: &[T]) {
        self.data[address..address + data.len()].copy_from_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import StackMemory and MemoryTrait for testing

    #[test]
    fn test_new_memory() {
        let mem = StackMemory::<u8, 16>::new(16, 42);
        assert_eq!(mem.get_size(), 16);

        // Ensure all values are initialized correctly
        for i in 0..16 {
            assert_eq!(*mem.read(i), 42);
        }
    }

    #[test]
    fn test_write_and_read() {
        let mut mem = StackMemory::<u8, 16>::new(16, 0);

        // Write a value and read it back
        mem.write(5, 42);
        assert_eq!(*mem.read(5), 42);
    }

    #[test]
    fn test_write_n_and_read_n() {
        let mut mem = StackMemory::<u8, 16>::new(16, 0);
        let values = [10, 20, 30, 40];

        // Write multiple values
        mem.write_n(4, &values);
        assert_eq!(mem.read_n(4, 4), &values);
    }

    #[test]
    #[should_panic]
    fn test_out_of_bounds_read() {
        let mem = StackMemory::<u8, 16>::new(16, 0);
        let _ = mem.read(16); // This should panic (out-of-bounds access)
    }

    #[test]
    #[should_panic]
    fn test_out_of_bounds_write() {
        let mut mem = StackMemory::<u8, 16>::new(16, 0);
        mem.write(16, 100); // Should panic (out-of-bounds)
    }

    #[test]
    #[should_panic]
    fn test_out_of_bounds_write_n() {
        let mut mem = StackMemory::<u8, 16>::new(16, 0);
        let data = [1, 2, 3, 4, 5];
        mem.write_n(14, &data); // Should panic (not enough space left)
    }
}
