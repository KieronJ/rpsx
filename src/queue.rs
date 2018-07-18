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

    pub fn empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn has_data(&self) -> bool {
        self.data.len() != 0
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
        match self.has_data() {
            false => 0,
            true => self.data.remove(0),
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
        match self.has_data() {
            false => 0,
            true => self.data.remove(0),
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
        let data;

        if self.has_data() {
            data = self.data.remove(0);
        } else {
            data = 0;
        }

        data
    }
}