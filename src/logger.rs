use std::{
    env,
    fmt::Display,
    io::{Write, stderr},
    str::FromStr,
};

use lazy_static::lazy_static;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevel {
    Debug = 0,
    #[default]
    Info = 1,
    Warning = 2,
    Error = 3,
}

impl FromStr for LogLevel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "debug" => Self::Debug,
            "info" => Self::Info,
            "warning" => Self::Warning,
            "error" => Self::Error,
            _ => todo!("Handle unrecognised log level"),
        })
    }
}

lazy_static! {
    static ref LOG_LEVEL: LogLevel = {
        const LOG_LEVEL_FLAG: &str = "--log-level";
        let commandline_args = env::args().collect::<Vec<String>>();
        let Some((index, _)) = commandline_args
            .iter()
            .enumerate()
            .find(|(_index, item)| item.as_str() == LOG_LEVEL_FLAG)
        else {
            return LogLevel::default();
        };

        let Some(log_level) = commandline_args.get(index + 1) else {
            return LogLevel::default();
        };

        let Ok(log_level) = LogLevel::from_str(log_level) else {
            return LogLevel::default();
        };

        log_level
    };
}

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[0;33m";
const BLUE: &str = "\x1b[0;34m";
const RESET: &str = "\x1b[0m";

#[allow(dead_code)]
pub fn debug(message: impl Display) {
    if *LOG_LEVEL > LogLevel::Debug {
        return;
    }
    let message = format!("{BLUE}[Debug]{RESET}: {message}");
    stderr()
        .write_all(message.as_bytes())
        .expect("Logging failed");
}

#[allow(dead_code)]
pub fn info(message: impl Display) {
    if *LOG_LEVEL > LogLevel::Info {
        return;
    }
    let message = format!("{GREEN}[Info]{RESET}: {message}");
    stderr()
        .write_all(message.as_bytes())
        .expect("Logging failed");
}

#[allow(dead_code)]
pub fn warn(message: impl Display) {
    if *LOG_LEVEL > LogLevel::Warning {
        return;
    }
    let message = format!("{YELLOW}[Warning]{RESET}: {message}");
    stderr()
        .write_all(message.as_bytes())
        .expect("Logging failed");
}

#[allow(dead_code)]
pub fn error(message: impl Display) {
    let message = format!("{RED}[Error]{RESET}: {message}");
    stderr()
        .write_all(message.as_bytes())
        .expect("Logging failed");
}
