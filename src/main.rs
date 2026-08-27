mod bar;
mod config;
mod logger;
mod modules;

use std::path::PathBuf;
use std::process::exit;
use std::sync::LazyLock;
use std::{env, fs};

use iced::{Font, daemon};

use log::{error, info};
use toml::Table;

use bar::Bar;

use crate::logger::Logger;

struct BarParameter {
    font_name: &'static str,
    bar_size: u32,
}

const DEFAULT_BAR_SIZE: u32 = 32;
const DEFAULT_FONT_NAME: &str = "monospace";

// Cramming the configuration table inside a lazy lock makes config hot reloading impossible but it
// allows me to side step a few problems such as not being able to adjust the default font and bar's
// height at runtime. The latter could be solved by parsing the config file before the bar launches
// but that will require parsing the same config file twice.
pub static CONFIG: LazyLock<Table> = LazyLock::new(|| {
    let config_file = get_config_location();
    info!("Using config from {}", config_file.display());

    let Ok(content) = fs::read_to_string(config_file) else {
        return Table::default();
    };
    content.parse::<Table>().unwrap_or_default()
});

static BAR_PARAMETER: LazyLock<BarParameter> = LazyLock::new(|| {
    let Some(bar_config) = CONFIG.get("bar") else {
        return BarParameter::default();
    };

    let font_name = || -> Option<&str> {
        let value = bar_config.get("font")?;
        value.as_str()
    }()
    .unwrap_or(DEFAULT_FONT_NAME);

    let bar_size = || -> Option<u32> {
        let value = bar_config.get("size")?;
        let int = value.as_integer()?;
        u32::try_from(int).ok()
    }()
    .unwrap_or(DEFAULT_BAR_SIZE);

    BarParameter {
        font_name,
        bar_size,
    }
});

impl Default for BarParameter {
    fn default() -> Self {
        Self {
            font_name: DEFAULT_FONT_NAME,
            bar_size: DEFAULT_BAR_SIZE,
        }
    }
}

fn main() -> iced::Result {
    let _ = log::set_logger(&Logger).map(|()| log::set_max_level(log::LevelFilter::Info));
    let settings = iced::Settings {
        id: Some("minibar".to_string()),
        default_font: Font::with_name(DEFAULT_FONT_NAME),
        exit_on_close_request: false,
        is_daemon: true,
        ..Default::default()
    };
    daemon(Bar::start, Bar::update, Bar::view)
        .settings(settings)
        .subscription(Bar::subscription)
        .default_font(Font::with_name(BAR_PARAMETER.font_name))
        .theme(Bar::theme)
        .run()
}

fn get_config_location() -> PathBuf {
    const CONFIG_PATH_FLAG: &str = "--config-file";
    const CONFIG_FILE: &str = "config.toml";

    let commandline_args: Vec<String> = env::args().collect();
    if let Some((index, _item)) = commandline_args
        .iter()
        .enumerate()
        .find(|(_index, item)| *item == CONFIG_PATH_FLAG)
    {
        if let Some(path) = commandline_args.get(index + 1) {
            return path.into();
        }

        error!("--config-file must be followed by a path to a config file, exiting now");
        exit(1);
    }

    let config_dir = get_config_dir();
    if !config_dir.exists()
        && let Err(err) = fs::create_dir_all(&config_dir)
    {
        error!("Unable to create config directory due to {err}, exiting now");
        exit(1);
    }

    config_dir.join(CONFIG_FILE)
}

fn get_config_dir() -> PathBuf {
    const CONFIG_DIR: &str = "minibar";

    if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config_home).join(CONFIG_DIR);
    }

    if let Some(home_dir) = env::home_dir() {
        return home_dir.join(".config").join(CONFIG_DIR);
    }

    error!(
        "Could not figure out how to get config directory. This issue may be resolved by setting $XDG_CONFIG_HOME or $HOME",
    );
    exit(1)
}
