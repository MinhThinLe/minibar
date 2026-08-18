mod bar;
mod modules;

use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::{Settings, application};

use bar::Bar;

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
        .subscription(Bar::subscription)
        .run()
}
