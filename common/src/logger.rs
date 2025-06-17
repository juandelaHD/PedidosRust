use colored::*;

#[derive(Debug, Clone)]
pub struct Logger {
    pub name: String,
}

impl Logger {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into().to_uppercase(),
        }
    }

    pub fn info(&self, msg: impl AsRef<str>) {
        println!(
            "{} {}",
            format!("[INFO][{}]", self.name).bold().bright_green(),
            msg.as_ref()
        );
    }

    pub fn warn(&self, msg: impl AsRef<str>) {
        println!(
            "{} {}",
            format!("[WARN][{}]", self.name).bold().yellow(),
            msg.as_ref()
        );
    }

    pub fn error(&self, msg: impl AsRef<str>) {
        eprintln!(
            "{} {}",
            format!("[ERROR][{}]", self.name)
                .bold()
                .bright_red()
                .blink(),
            msg.as_ref()
        );
    }
}
