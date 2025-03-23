use crate::memory::memory_trait::MemoryTrait;

pub struct HeapMemory<T: Sized + Copy> {
    size: usize,
    data: Vec<T>,
}

impl<T: Sized + Copy> MemoryTrait<T> for HeapMemory<T> {
    fn new(size: usize, default: T) -> Self {
        Self {
            size,
            data: vec![default; size],
        }
    }

    fn get_size(&self) -> usize {
        self.data.len()
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
