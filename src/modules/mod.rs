mod battery;
mod clock;
mod cpu;
mod group;
mod memory;
mod pipewire;
mod script;
mod systray;
mod temperature;
mod workspaces;

use std::fmt::Debug;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use downcast_rs::{DowncastSync, impl_downcast};
use iced::border::Radius;
use iced::futures::SinkExt;
use iced::futures::channel::mpsc::Sender;
use iced::{Border, Color, Element, Padding, Subscription};
use toml::value::Array;
use toml::{Table, Value};

use crate::CONFIG;
use crate::bar::{BarEvent, Output};
use crate::modules::module_id::ModuleId;

pub mod reexports {
    pub use super::Module;
    pub use super::battery::Battery;
    pub use super::clock::Clock;
    pub use super::cpu::Cpu;
    pub use super::group::Group;
    pub use super::memory::Memory;
    pub use super::pipewire::PipeWire;
    pub use super::script::Script;
    pub use super::systray::SysTray;
    pub use super::temperature::Temperature;
    pub use super::workspaces::Workspaces;
}

mod module_id {
    use std::sync::atomic::{AtomicU64, Ordering};

    static MODULE_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ModuleId(u64);

    pub(super) fn module_id_unique() -> ModuleId {
        ModuleId(MODULE_ID.fetch_add(1, Ordering::Relaxed))
    }

    #[test]
    fn module_id_unique_is_actually_unique() {
        assert_ne!(module_id_unique(), module_id_unique())
    }
}

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct ModuleUpdate(pub ModuleId, pub Arc<dyn ModuleData>);

#[derive(Debug, Default)]
pub struct CommonStyle {
    pub padding: Padding,
    pub border: Border,
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}

pub trait ModuleData: DowncastSync {}
impl_downcast!(sync ModuleData);

pub trait Module: DowncastSync {
    fn update(&mut self, module_update: ModuleUpdate);
    fn view(&self, output: &Output) -> Element<'_, BarEvent>;
    fn subscription(&self) -> Option<Subscription<ModuleUpdate>>;
    fn id(&self) -> &[ModuleId];
    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized;
}

pub trait NamedModule {
    fn name() -> &'static str;
}

impl From<&Table> for CommonStyle {
    fn from(value: &Table) -> Self {
        const DEFAULT_PADDING: Padding = Padding::ZERO;

        let padding = || -> Option<Padding> {
            let padding = value.get("padding")?;
            parse_padding(padding)
        }()
        .unwrap_or(DEFAULT_PADDING);

        let border = value.get("border").map_or(Border::default(), parse_border);

        let background = get_color(value, "background");
        let foreground = get_color(value, "foreground");

        Self {
            padding,
            border,
            background,
            foreground,
        }
    }
}

fn get_poll_interval(module_name: &str) -> Duration {
    || -> Option<Duration> {
        let module = CONFIG.get(module_name)?;
        let poll_interval = module.get("poll_interval")?;
        let poll_interval = poll_interval.as_integer()?;
        Some(Duration::from_millis(poll_interval.cast_unsigned()))
    }()
    .unwrap_or(DEFAULT_POLL_INTERVAL)
}

fn parse_border(value: &Value) -> Border {
    let Some(value) = value.as_table() else {
        return Border::default();
    };

    let color = get_color(value, "color").unwrap_or(Color::BLACK);
    let width = get_float(value, "width").unwrap_or_default();
    let radius = get_float(value, "radius").unwrap_or_default();

    Border {
        color,
        width,
        radius: Radius::from(radius),
    }
}

fn to_icon_list(icons_str: &str) -> Vec<char> {
    icons_str
        .chars()
        .filter(|char| !char.is_whitespace())
        .collect()
}

fn get_icon(icon_list: &[char], value: u16, max: u16) -> char {
    if icon_list.is_empty() {
        return char::default();
    }

    let levels = icon_list.len() as u16;
    let step_size = max / levels;

    for level in 1..levels {
        if value < level * step_size {
            return icon_list[level as usize - 1];
        }
    }

    icon_list[icon_list.len() - 1]
}

fn rgba8_to_color(raw: u32) -> Color {
    let bytes = raw.to_be_bytes();
    let red = bytes[0];
    let green = bytes[1];
    let blue = bytes[2];
    let alpha = f32::from(bytes[3]) / f32::from(u8::MAX);

    Color::from_rgba8(red, green, blue, alpha)
}

fn parse_padding(value: &Value) -> Option<Padding> {
    match value {
        Value::Integer(integer) => {
            if *integer < 0 {
                None
            } else {
                Some(Padding::from((*integer).try_into().unwrap_or(0)))
            }
        }
        Value::Float(float) => {
            if *float < 0.0 {
                None
            } else {
                Some(Padding::from(*float as f32))
            }
        }
        Value::Array(array) => padding_from_array(array),
        Value::Table(table) => Some(padding_from_table(table)),

        _ => None,
    }
}

fn padding_from_table(value: &Table) -> Padding {
    let top = get_float(value, "top").unwrap_or_default();
    let right = get_float(value, "right").unwrap_or_default();
    let bottom = get_float(value, "bottom").unwrap_or_default();
    let left = get_float(value, "left").unwrap_or_default();

    Padding {
        top,
        right,
        bottom,
        left,
    }
}

fn padding_from_array(value: &Array) -> Option<Padding> {
    if value.len() != 2 && value.len() != 4 {
        return None;
    }
    if value.len() == 2 {
        let padding = [float_from_value(&value[0])?, float_from_value(&value[1])?];
        return Some(Padding::from(padding));
    }
    if value.len() == 4 {
        return Some(Padding {
            top: float_from_value(&value[0])?,
            right: float_from_value(&value[1])?,
            bottom: float_from_value(&value[2])?,
            left: float_from_value(&value[3])?,
        });
    }

    unreachable!()
}

fn float_from_value(value: &Value) -> Option<f32> {
    match value {
        Value::Float(float) => Some(*float as f32),
        Value::Integer(integer) => Some(*integer as f32),
        _ => None,
    }
}

fn get_str<'a>(table: &'a Table, key: &'a str) -> Option<&'a str> {
    let string = table.get(key)?;
    string.as_str()
}

fn get_int(table: &Table, key: &str) -> Option<i64> {
    let int = table.get(key)?;
    int.as_integer()
}

fn get_color(table: &Table, key: &str) -> Option<Color> {
    let color = get_int(table, key)?;
    let raw_rgba8 = u32::try_from(color).ok()?;
    Some(rgba8_to_color(raw_rgba8))
}

fn get_float(value: &Table, index_key: &str) -> Option<f32> {
    float_from_value(value.get(index_key)?)
}

fn value_from_file<T: FromStr>(path: impl AsRef<Path>) -> Option<T> {
    use std::fs::read_to_string;

    read_to_string(path).ok()?.trim().parse::<T>().ok()
}

fn float_to_string(float: f32) -> String {
    format!("{float:.1}")
}

async fn send_data<T: ModuleData>(
    sender: &mut Sender<ModuleUpdate>,
    destination: ModuleId,
    data: T,
) {
    let packet = ModuleUpdate(destination, Arc::new(data));
    sender.send(packet).await.expect("Broken pipe");
}

#[cfg(test)]
mod test {
    use crate::modules::*;
    use iced::{Border, Color, Padding, border::Radius};
    use toml::Table;

    const PADDING_REPRESENTATION_1: &str = "
        padding = {
            left = 1.0,
            right = 2.0,
        }
    ";
    const PADDING_REPRESENTATION_2: &str = "
        padding = [1.0, 2]
    ";
    const PADDING_REPRESENTATION_3: &str = "
        padding = 3
    ";

    #[test]
    fn test_parse_padding() {
        let representation_1 = PADDING_REPRESENTATION_1.parse::<Table>().unwrap();
        let table_1 = representation_1["padding"].clone();
        assert_eq!(
            parse_padding(&table_1).unwrap(),
            padding_from_table(&table_1.as_table().unwrap())
        );

        let representation_1 = padding_from_table(table_1.as_table().unwrap());
        assert_eq!(
            representation_1,
            Padding {
                left: 1.0,
                right: 2.0,
                ..Default::default()
            }
        );

        let representation_2 = PADDING_REPRESENTATION_2.parse::<Table>().unwrap();
        let table_2 = representation_2["padding"].clone();
        assert_eq!(
            parse_padding(&table_2),
            padding_from_array(table_2.as_array().unwrap())
        );

        let representation_2 = padding_from_array(representation_2["padding"].as_array().unwrap());
        assert_eq!(representation_2.unwrap(), Padding::from([1, 2]));

        let representation_3 = PADDING_REPRESENTATION_3.parse::<Table>().unwrap();
        let representation_3 = parse_padding(&representation_3["padding"]);
        assert_eq!(representation_3.unwrap(), Padding::from(3));
    }

    const BORDER_REPRESENTATION: &str = "
        border = {
            color = 0xff00ff00,
            width = 2,
            radius = 3,
        }
    ";

    #[test]
    fn test_parse_border() {
        let border_table = BORDER_REPRESENTATION.parse::<Table>().unwrap();
        assert_eq!(
            parse_border(&border_table["border"]),
            Border {
                color: Color::from_rgba8(255, 0, 255, 0.0),
                width: 2.0,
                radius: Radius::from(3)
            }
        );
    }

    #[test]
    fn test_parse_color() {
        let color: u32 = 0xff00ff00;
        assert_eq!(Color::from_rgba8(255, 0, 255, 0.0), rgba8_to_color(color));
    }
}
