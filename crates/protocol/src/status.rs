use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatusMessage {
    Empty,
    Text { text: String },
}

impl StatusMessage {
    pub fn append_to(&self, target: &mut String) {
        match self {
            StatusMessage::Empty => {}
            StatusMessage::Text { text } => target.push_str(text),
        }
    }

    pub fn append_to_status_line(&self, target: &mut String) {
        match self {
            StatusMessage::Empty => {}
            StatusMessage::Text { text } => {
                target.push_str(" > ");
                target.push_str(text);
            }
        }
    }

    pub fn or(self, fallback: StatusMessage) -> StatusMessage {
        match self {
            StatusMessage::Empty => fallback,
            StatusMessage::Text { .. } => self,
        }
    }

    pub fn store(&mut self, message: StatusMessage) {
        *self = message;
    }

    pub fn clear(&mut self) {
        *self = StatusMessage::Empty;
    }
}
