use chrono::Local;
use colored::*;

#[derive(Debug, Clone)]
pub struct Logger {
    pub name: String,
    pub info_color: Color,
}

impl Logger {
    pub fn new(name: impl Into<String>, info_color: Color) -> Self {
        Self {
            name: name.into().to_uppercase(),
            info_color,
        }
    }

    fn timestamp() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    pub fn info(&self, msg: impl AsRef<str>) {
        println!(
            "{} {} {}",
            format!("[{}][INFO][{}]", Self::timestamp(), self.name)
                .bold()
                .color(self.info_color),
            "→".dimmed(),
            msg.as_ref()
        );
    }

    pub fn warn(&self, msg: impl AsRef<str>) {
        println!(
            "{} {} {}",
            format!("[{}][WARN][{}]", Self::timestamp(), self.name)
                .bold()
                .yellow(),
            "→".dimmed(),
            msg.as_ref()
        );
    }

    pub fn error(&self, msg: impl AsRef<str>) {
        eprintln!(
            "{} {} {}",
            format!("[{}][ERROR][{}]", Self::timestamp(), self.name)
                .bold()
                .bright_red()
                .blink(),
            "→".dimmed(),
            msg.as_ref()
        );
    }
}
