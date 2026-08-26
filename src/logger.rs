use std::io::{Write, stderr};

use log::{Level, Log};

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        stderr()
            .write_all(format!("{}: {}\n", color_record(record.level()), record.args()).as_bytes())
            .expect("Could not write to stderr");
    }

    fn flush(&self) {}
}

fn color_record(log_level: Level) -> String {
    const ERROR: &str = "[Error]";
    const WARNING: &str = "[Warning]";
    const INFO: &str = "[Info]";
    const DEBUG: &str = "[Debug]";
    const TRACE: &str = "[Trace]";

    const RED: &str = "\x1b[0;31m";
    const GREEN: &str = "\x1b[0;32m";
    const YELLOW: &str = "\x1b[0;33m";
    const BLUE: &str = "\x1b[0;34m";
    const RESET: &str = "\x1b[0m";
    match log_level {
        Level::Error => format!("{RED}{ERROR}{RESET}"),
        Level::Warn => format!("{YELLOW}{WARNING}{RESET}"),
        Level::Info => format!("{GREEN}{INFO}{RESET}"),
        Level::Debug => format!("{BLUE}{DEBUG}{RESET}"),
        Level::Trace => TRACE.to_string(),
    }
}
