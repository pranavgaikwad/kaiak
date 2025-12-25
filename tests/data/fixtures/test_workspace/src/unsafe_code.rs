use std::mem;

pub struct UnsafeExample {
    data: Vec<u8>,
}

impl UnsafeExample {
    pub fn new() -> Self {
        Self {
            data: vec![1, 2, 3, 4, 5],
        }
    }

    pub fn get_raw_ptr(&self) -> *const u8 {
        unsafe {
            // This unsafe block could be improved
            mem::transmute(&self.data[0])
        }
    }

    pub fn safe_alternative(&self) -> &[u8] {
        &self.data
    }
}