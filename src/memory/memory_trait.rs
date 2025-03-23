pub trait MemoryTrait<T: Sized + Copy> {
    fn new(size: usize, default: T) -> Self
        where
            Self: Sized;

    fn get_size(&self) -> usize;
    fn read(&self, address: usize) -> &T;
    fn read_n(&self, address: usize, n: usize) -> &[T];
    fn write(&mut self, address: usize, data: T);
    fn write_n(&mut self, address: usize, data: &[T]);
}