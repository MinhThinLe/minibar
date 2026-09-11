use chrono::Local;

use super::prelude::*;

const DEFAULT_FORMAT: &str = "%R";

struct ClockConfig {
    format: Box<str>,
}

#[derive(NamedModule)]
pub struct Clock {
    config: ClockConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for Clock {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        let current_date = Local::now();
        container(text(current_date.format(&self.config.format).to_string()))
            .padding(self.style.padding)
            .style(|_idk| container::Style {
                text_color: self.style.foreground,
                background: self.style.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, _module_update: ModuleUpdate) {}

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        None
    }

    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();
        let style = CommonStyle::from(table);

        let config = ClockConfig { format };

        let id = [module_id_unique()];

        Box::new(Self { config, style, id })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }
}
