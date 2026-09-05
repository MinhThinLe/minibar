use std::str::FromStr;
use std::thread::sleep;

use iced::futures::Stream;
use iced::widget::{container, text};
use iced::{Background, Color, Element, Subscription, stream};
use minibar_derives::{ModuleData, NamedModule};
use toml::Table;

use crate::modules::module_id::module_id_unique;

use super::*;

const DEFAULT_FORMAT: &str = "BAT: {percentage}%";

#[derive(Default, Clone, Copy, Eq, PartialEq)]
enum BatteryState {
    Charging,
    Discharging,
    #[default]
    Plugged,
}

#[derive(Default, Clone, Copy, Eq, PartialEq, ModuleData)]
struct BatteryStatus {
    pub percentage: u8,
    pub state: BatteryState,
}

struct BatteryConfig {
    format: Box<str>,
    critical_threshold: u8,
    critical_foreground: Option<Color>,
}

#[derive(NamedModule)]
pub struct Battery {
    status: BatteryStatus,
    config: BatteryConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl FromStr for BatteryState {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "discharging" => Self::Discharging,
            "full" | "not charging" => Self::Plugged,
            "charging" => Self::Charging,
            _ => return Err(()),
        })
    }
}

impl Battery {
    fn get_text(&self) -> String {
        const PERCENTAGE: &str = "{percentage}";
        self.config
            .format
            .replace(PERCENTAGE, &self.status.percentage.to_string())
    }

    fn get_color(&self) -> Option<Color> {
        if self.status.percentage <= self.config.critical_threshold {
            return self.config.critical_foreground;
        }

        self.style.foreground
    }

    fn get_background(&self) -> Option<Background> {
        Some(Background::Color(self.style.background?))
    }
}

impl Module for Battery {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        container(text(self.get_text()))
            .padding(self.style.padding)
            .style(|_idk| container::Style {
                text_color: self.get_color(),
                background: self.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, module_update: ModuleUpdate) {
        let ModuleUpdate(_module_id, update_data) = module_update;
        let update = update_data
            .downcast_ref::<BatteryStatus>()
            .expect("A bug in the routing logic");
        self.status = *update;
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run_with(self.id[0], |destination| {
            worker(*destination)
        }))
    }

    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT);
        let critical_threshold = get_int(table, "critical_threshold").unwrap_or_default() as u8;
        let critical_foreground = get_color(table, "ciritical_foreground");
        let style = CommonStyle::from(table);

        let config = BatteryConfig {
            format: format.into(),
            critical_threshold,
            critical_foreground,
        };

        let id = [module_id_unique()];

        Box::new(Self {
            config,
            style,
            status: BatteryStatus::default(),
            id,
        })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }
}

fn read_battery_info() -> BatteryStatus {
    const BATTERY_PATH: &str = "/sys/class/power_supply";
    const BATTERY: &str = "BAT0";
    const CAPACITY: &str = "capacity";
    const STATUS: &str = "status";

    let battery_directory = format!("{BATTERY_PATH}/{BATTERY}");

    let percentage = value_from_file(format!("{battery_directory}/{CAPACITY}")).unwrap_or_default();
    let state = value_from_file(format!("{battery_directory}/{STATUS}")).unwrap_or_default();

    BatteryStatus { percentage, state }
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let poll_interval = get_poll_interval("battery");
        let mut content = read_battery_info();

        send_data(&mut output, module_id, content).await;
        loop {
            sleep(poll_interval);
            let new_content = read_battery_info();

            if new_content == content {
                continue;
            }

            send_data(&mut output, module_id, content).await;
            content = new_content;
        }
    })
}
