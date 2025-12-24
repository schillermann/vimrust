use std::io;

use vimrust_protocol::StatusMessage;

pub struct ProtocolGate {
    expected: u32,
    status: StatusMessage,
}

impl ProtocolGate {
    pub fn new(expected: u32) -> Self {
        Self {
            expected,
            status: StatusMessage::Empty,
        }
    }

    pub fn observe(&mut self, actual: u32) {
        if self.expected == actual {
            self.status = StatusMessage::Empty;
        } else {
            self.status = StatusMessage::Text {
                text: format!("protocol mismatch: core {} ui {}", actual, self.expected),
            };
        }
    }

    pub fn status(&self) -> StatusMessage {
        self.status.clone()
    }

    pub fn report(&self) {
        let mut message = String::new();
        self.status.append_to(&mut message);
        if !message.is_empty() {
            eprintln!("vimrust: {}", message);
        }
    }

    pub fn result(&self) -> io::Result<()> {
        match &self.status {
            StatusMessage::Empty => Ok(()),
            StatusMessage::Text { text } => Err(io::Error::new(io::ErrorKind::Other, text.clone())),
        }
    }
}
