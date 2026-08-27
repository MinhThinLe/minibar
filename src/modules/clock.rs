use chrono::Local;
use iced::{
    Background,
    widget::{container, text},
};

use super::*;

const DEFAULT_FORMAT: &str = "%R";

struct ClockConfig {
    format: Box<str>,
}

pub struct Clock {
    config: ClockConfig,
    style: CommonStyle,
}

impl Module for Clock {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        let current_date = Local::now();
        container(text(current_date.format(&self.config.format).to_string()))
            .padding(self.style.padding)
            .style(|_idk| container::Style {
                text_color: self.style.foreground,
                background: self.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, _update_data: Arc<dyn ModuleData>) {}

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        None
    }

    fn new_or_default(table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();
        let style = CommonStyle::from(table);

        let config = ClockConfig { format };

        Rc::new(Self { config, style })
    }
}

impl Clock {
    fn get_background(&self) -> Option<Background> {
        Some(Background::Color(self.style.background?))
    }
}
