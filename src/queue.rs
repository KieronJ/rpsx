pub struct Queue<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T> Queue<T> {
    pub fn push(&mut self, value: T) {
        if !self.full() {
            self.data.push(value);
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    pub fn full(&self) -> bool {
        self.data.len() >= self.capacity
    }

    pub fn has_space(&self) -> bool {
        self.data.len() < self.capacity
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn data(&mut self) -> &mut Vec<T> {
        &mut self.data
    }
}

impl Queue<u8> {
    pub fn new(capacity: usize) -> Queue<u8> {
        Queue {
            data: Vec::with_capacity(capacity),
            capacity: capacity,
        }
    }

    pub fn pop(&mut self) -> u8 {
        if self.has_data() {
            return self.data.remove(0);
        } else {
            return 0;
        }
    }
}

impl Queue<u16> {
    pub fn new(capacity: usize) -> Queue<u16> {
        Queue {
            data: Vec::with_capacity(capacity),
            capacity: capacity,
        }
    }

    pub fn pop(&mut self) -> u16 {
        if self.has_data() {
            return self.data.remove(0);
        } else {
            return 0;
        }
    }
}

impl Queue<u32> {
    pub fn new(capacity: usize) -> Queue<u32> {
        Queue {
            data: Vec::with_capacity(capacity),
            capacity: capacity,
        }
    }

    pub fn pop(&mut self) -> u32 {
        if self.has_data() {
            return self.data.remove(0);
        } else {
            return 0;
        }
    }
}
