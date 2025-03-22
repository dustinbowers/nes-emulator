pub struct Memory {
    size: usize,
    data: Box<[u8]>
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            data: vec![0; size].into_boxed_slice()
        }
    }

    pub fn get_size(self: &Self) -> usize {
        self.data.len()
    }

    pub fn read(self: &Self, address: usize) -> u8 {
        self.data[address]
    }

    pub fn write(self: &mut Self, address: usize, byte: u8) {
        self.data[address] = byte
    }

}