pub enum EventType {
    Vblank,
    Cdrom,
    Dma,
    Timer(usize),
    Controller,
    Spu,
}

struct Event {
    event_type: EventType,
    timestamp: i64,
}

pub struct Scheduler {
    events: Vec<Event>,
    timestamp: i64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            timestamp: 0,
        }
    }

    pub fn add_ticks(&mut self, ticks: i64) {
        self.timestamp += ticks;
    }

    pub fn next_timestamp(&self) -> i64 {
        assert!(!self.events.is_empty());

        let timestamp = self.events[0].timestamp - self.timestamp;
        assert!(timestamp >= 0);

        timestamp
    }

    pub fn add_event(&mut self, event_type: EventType, ticks: i64) {
        assert!(ticks >= 0);

        self.events.push(Event {
            event_type: event_type,
            timestamp: self.timestamp + ticks,
        });

        self.events.sort_by(|a, b| { a.timestamp.cmp(&b.timestamp) });
    }

    pub fn remove_event(&mut self) {
        
    }
}