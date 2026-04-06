// =============================================================================
// Florynx Kernel — IPC: Message Passing
// =============================================================================

use alloc::collections::VecDeque;
use alloc::string::String;

/// An IPC message sent between tasks.
#[derive(Debug, Clone)]
pub struct Message {
    /// Sender task ID.
    pub sender: u64,
    /// Receiver task ID.
    pub receiver: u64,
    /// Message type tag.
    pub msg_type: MessageType,
    /// Payload data.
    pub payload: MessagePayload,
}

/// Type of IPC message.
#[derive(Debug, Clone)]
pub enum MessageType {
    Data,
    Signal,
    Request,
    Response,
}

/// Message payload variants.
#[derive(Debug, Clone)]
pub enum MessagePayload {
    Empty,
    Text(String),
    Bytes(alloc::vec::Vec<u8>),
    Integer(i64),
}

impl Message {
    pub fn new_text(sender: u64, receiver: u64, text: &str) -> Self {
        Message {
            sender,
            receiver,
            msg_type: MessageType::Data,
            payload: MessagePayload::Text(String::from(text)),
        }
    }

    pub fn new_signal(sender: u64, receiver: u64) -> Self {
        Message {
            sender,
            receiver,
            msg_type: MessageType::Signal,
            payload: MessagePayload::Empty,
        }
    }
}

/// Simple message queue (mailbox).
pub struct MessageQueue {
    queue: VecDeque<Message>,
    capacity: usize,
}

impl MessageQueue {
    pub fn new(capacity: usize) -> Self {
        MessageQueue {
            queue: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn send(&mut self, msg: Message) -> Result<(), &'static str> {
        if self.queue.len() >= self.capacity {
            return Err("message queue full");
        }
        self.queue.push_back(msg);
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
