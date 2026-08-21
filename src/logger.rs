use std::{
    env,
    fmt::Display,
    io::{Write, stderr},
    str::FromStr,
    sync::LazyLock,
};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

static LOG_LEVEL: LazyLock<LogLevel> = LazyLock::new(|| {
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
});

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[0;33m";
const BLUE: &str = "\x1b[0;34m";
const RESET: &str = "\x1b[0m";

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "{BLUE}[Debug]{RESET}: "),
            LogLevel::Info => write!(f, "{GREEN}[Info]{RESET}: "),
            LogLevel::Warning => write!(f, "{YELLOW}[Warning]{RESET}: "),
            LogLevel::Error => write!(f, "{RED}[Error]{RESET}: "),
        }
    }
}

fn generate_message(log_level: LogLevel, message: impl Display) -> String {
    format!("{log_level}{message}\n")
}

fn report(message: &str) {
    stderr()
        .write_all(message.as_bytes())
        .expect("Unable to write to stderr");
}

#[allow(dead_code)]
pub fn debug(message: impl Display) {
    if *LOG_LEVEL > LogLevel::Debug {
        return;
    }
    report(&generate_message(LogLevel::Debug, message));
}

#[allow(dead_code)]
pub fn info(message: impl Display) {
    if *LOG_LEVEL > LogLevel::Info {
        return;
    }
    report(&generate_message(LogLevel::Info, message));
}

#[allow(dead_code)]
pub fn warn(message: impl Display) {
    if *LOG_LEVEL > LogLevel::Warning {
        return;
    }
    report(&generate_message(LogLevel::Warning, message));
}

#[allow(dead_code)]
pub fn error(message: impl Display) {
    report(&generate_message(LogLevel::Error, message));
}
