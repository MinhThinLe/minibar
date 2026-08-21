pub mod battery;
pub mod cpu;

use std::fmt::Debug;
use std::rc::Rc;
use std::{any::TypeId, sync::Arc};

use downcast_rs::{DowncastSync, impl_downcast};
use iced::border::Radius;
use iced::{Border, Color, Element, Padding, Subscription};
use toml::value::Array;
use toml::{Table, Value};

use crate::bar::BarEvent;

pub struct ModuleUpdate(pub TypeId, pub Arc<dyn ModuleData>);

impl Debug for ModuleUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Update designated for module {:?}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct CommonStyle {
    pub padding: Padding,
    pub border: Border,
    pub background: Color,
    pub foreground: Color,
}

pub trait ModuleData: DowncastSync {}
impl_downcast!(sync ModuleData);

pub trait Module: DowncastSync + Debug {
    fn update(&mut self, update_data: Arc<dyn ModuleData>);
    fn view(&self) -> Element<'_, BarEvent>;
    fn subscription(&self) -> Option<Subscription<ModuleUpdate>>;
    fn try_new(table: &Table) -> Option<Rc<dyn Module>>
    where
        Self: Sized;
}

impl From<&Table> for CommonStyle {
    fn from(value: &Table) -> Self {
        const DEFAULT_PADDING: Padding = Padding::ZERO;
        const DEFAULT_BACKGROUND: Color = Color::BLACK;
        const DEFAULT_FOREGROUND: Color = Color::WHITE;

        let padding = value.get("padding").map_or(DEFAULT_PADDING, |value| {
            parse_padding(value).unwrap_or(DEFAULT_PADDING)
        });
        let border = value.get("border").map_or(Border::default(), parse_border);

        Self {
            padding,
            border,
            background: DEFAULT_BACKGROUND,
            foreground: DEFAULT_FOREGROUND,
        }
    }
}

fn parse_border(value: &Value) -> Border {
    let color = value.get("color").map_or(Color::BLACK, |color| {
        color
            .as_integer()
            .map_or(Color::BLACK, |color| rgba8_to_color(color as u32))
    });
    let width = value
        .get("width")
        .map_or(0.0, |width| float_from_value(width).unwrap_or_default());
    let radius = value
        .get("radius")
        .map_or(0.0, |radius| float_from_value(radius).unwrap_or_default());

    Border {
        color,
        width,
        radius: Radius::from(radius),
    }
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
    let top = value
        .get("top")
        .map_or(0.0, |value| float_from_value(value).unwrap_or_default());
    let right = value
        .get("right")
        .map_or(0.0, |value| float_from_value(value).unwrap_or_default());
    let bottom = value
        .get("bottom")
        .map_or(0.0, |value| float_from_value(value).unwrap_or_default());
    let left = value
        .get("left")
        .map_or(0.0, |value| float_from_value(value).unwrap_or_default());

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
