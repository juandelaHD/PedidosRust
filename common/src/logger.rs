#[derive(Clone)]
pub struct Logger {
    pub name: String,
}

impl Logger {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn info(&self, msg: impl AsRef<str>) {
        println!("[INFO][{}] {}", self.name, msg.as_ref());
    }

    pub fn warn(&self, msg: impl AsRef<str>) {
        println!("[WARN][{}] {}", self.name, msg.as_ref());
    }

    pub fn error(&self, msg: impl AsRef<str>) {
        eprintln!("[ERROR][{}] {}", self.name, msg.as_ref());
    }
}
