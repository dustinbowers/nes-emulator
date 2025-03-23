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
