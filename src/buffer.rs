pub struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns a mutable handle for queuing terminal commands.
    pub fn writer(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    /// Returns the buffered bytes for flushing to stdout.
    pub fn slice(&self) -> &[u8] {
        &self.data
    }

    pub fn changed(&self) -> bool {
        !self.data.is_empty()
    }
}
