// =============================================================================
// Florynx Kernel — IPC: Channels
// =============================================================================

use alloc::collections::VecDeque;

/// A bidirectional communication channel between two tasks.
pub struct Channel {
    id: u64,
    buffer_a_to_b: VecDeque<alloc::vec::Vec<u8>>,
    buffer_b_to_a: VecDeque<alloc::vec::Vec<u8>>,
    capacity: usize,
}

impl Channel {
    pub fn new(id: u64, capacity: usize) -> Self {
        Channel {
            id,
            buffer_a_to_b: VecDeque::with_capacity(capacity),
            buffer_b_to_a: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Send data from endpoint A to endpoint B.
    pub fn send_a_to_b(&mut self, data: alloc::vec::Vec<u8>) -> Result<(), &'static str> {
        if self.buffer_a_to_b.len() >= self.capacity {
            return Err("channel buffer full");
        }
        self.buffer_a_to_b.push_back(data);
        Ok(())
    }

    /// Receive data at endpoint B from endpoint A.
    pub fn recv_at_b(&mut self) -> Option<alloc::vec::Vec<u8>> {
        self.buffer_a_to_b.pop_front()
    }

    /// Send data from endpoint B to endpoint A.
    pub fn send_b_to_a(&mut self, data: alloc::vec::Vec<u8>) -> Result<(), &'static str> {
        if self.buffer_b_to_a.len() >= self.capacity {
            return Err("channel buffer full");
        }
        self.buffer_b_to_a.push_back(data);
        Ok(())
    }

    /// Receive data at endpoint A from endpoint B.
    pub fn recv_at_a(&mut self) -> Option<alloc::vec::Vec<u8>> {
        self.buffer_b_to_a.pop_front()
    }
}
