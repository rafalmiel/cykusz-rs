use mmio::VolBox;

pub struct VCell<T: Copy>(VolBox<T, mmio::Allow, mmio::Allow>);

impl<T: Copy> VCell<T> {
    pub fn get(&self) -> T {
        self.0.read()
    }

    pub fn set(&mut self, v: T) {
        self.0.write(v)
    }
}