pub struct Memory<T: Sized + Copy> {
    size: usize,
    data: Vec<T>,
}

impl<T: Sized + Copy> Memory<T> {
    pub fn new(size: usize, default: T) -> Self {
        Self {
            size,
            data: vec![default; size],
        }
    }

    pub fn get_size(&self) -> usize {
        self.data.len()
    }

    pub fn read(&self, address: usize) -> &T {
        &self.data[address]
    }

    pub fn read_n(&self, address: usize, n: usize) -> &[T] {
        &self.data[address..address + n]
    }

    pub fn write(&mut self, address: usize, data: T) {
        self.data[address] = data;
    }

    pub fn write_n(&mut self, address: usize, data: &[T]) {
        self.data[address..address + data.len()].copy_from_slice(data);
    }

    pub fn write_vec(&mut self, address: usize, data: Vec<T>) {
        self.data.splice(address..address + data.len(), data);
    }

}
