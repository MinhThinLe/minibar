use iced_layershell::{Settings, application};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{StartMode, LayerShellSettings};

use crate::bar::Bar;

mod bar;

fn main() -> iced_layershell::Result {
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

    application(Bar::default, Bar::namespace, Bar::update, Bar::view)
        .settings(settings)
        .run()
}
