mod bar;
mod config;
mod logger;
mod modules;

use std::path::PathBuf;
use std::process::exit;
use std::sync::LazyLock;
use std::{env, fs};

use iced::Font;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::{Settings, application};

use bar::Bar;
use toml::Table;

use crate::logger::error;

struct BarParameter {
    font_name: String,
    bar_size: u32,
}

const DEFAULT_BAR_SIZE: u32 = 32;
const DEFAULT_FONT_NAME: &str = "monospace";
impl Default for BarParameter {
    fn default() -> Self {
        Self {
            font_name: DEFAULT_FONT_NAME.to_string(),
            bar_size: DEFAULT_BAR_SIZE,
        }
    }
}

static BAR_PARAMETER: LazyLock<BarParameter> = LazyLock::new(|| {
    let config_file = get_config_location();
    let Ok(config_content) = fs::read_to_string(config_file) else {
        return BarParameter::default();
    };

    let Ok(config_table) = config_content.parse::<Table>() else {
        return BarParameter::default();
    };

    let Some(bar_config) = config_table.get("bar") else {
        return BarParameter::default();
    };

    let font_name = bar_config.get("font").map_or(DEFAULT_FONT_NAME, |value| {
        value.as_str().unwrap_or(DEFAULT_FONT_NAME)
    });

    let bar_size = bar_config.get("size").map_or(DEFAULT_BAR_SIZE, |value| {
        value.as_integer().unwrap_or(i64::from(DEFAULT_BAR_SIZE)) as u32
    });

    BarParameter {
        font_name: font_name.to_string(),
        bar_size,
    }
});

fn main() -> iced_layershell::Result {
    let start_mode = StartMode::Active;

    let layer_settings = LayerShellSettings {
        size: Some((BAR_PARAMETER.bar_size, BAR_PARAMETER.bar_size)),
        exclusive_zone: BAR_PARAMETER.bar_size.cast_signed(),
        anchor: Anchor::Top | Anchor::Left | Anchor::Right, // TODO: Vertical status bar support
        start_mode,
        ..Default::default()
    };

    let settings = Settings {
        layer_settings,
        ..Default::default()
    };

    application(Bar::start, Bar::namespace, Bar::update, Bar::view)
        .settings(settings)
        .subscription(Bar::subscription)
        .default_font(Font::with_name(&BAR_PARAMETER.font_name))
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

        error("--config-file must be followed by a path to a config file, exiting now");
        exit(1);
    }

    let config_dir = get_config_dir();
    if !config_dir.exists()
        && let Err(err) = fs::create_dir_all(&config_dir)
    {
        error(format!(
            "Unable to create config directory due to {err}, exiting now"
        ));
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

    error(
        "Could not figure out how to get config directory. This issue may be resolved by setting $XDG_CONFIG_HOME or $HOME",
    );
    exit(1)
}
