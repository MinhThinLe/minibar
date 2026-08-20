mod bar;
mod config;
mod logger;
mod modules;

use std::path::PathBuf;

use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::{Settings, application};

use bar::Bar;

fn main() -> iced_layershell::Result {
    // TODO: Make a bootstrap struct
    // A 2 stage initialization like this is necessary as there's currently no way to set the
    // application's default font on runtime. Iced 0.15 will make this obsolete and this code will
    // be refactored when iced 0.15 is released
    let width = 0;
    let heigth = 32;
    let start_mode = StartMode::Active;

    let layer_settings = LayerShellSettings {
        size: Some((width, heigth)),
        exclusive_zone: heigth as i32,
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
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
        .theme(Bar::theme)
        .run()
}

fn get_config_location() -> PathBuf {
    // TODO: Make this function less stupid
    PathBuf::from("/home/t0ast/.config/minibar/config.toml")
}
