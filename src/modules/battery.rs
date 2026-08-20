use std::any::TypeId;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use iced::futures::{SinkExt, Stream};
use iced::widget::{container, text};
use iced::{Element, Subscription, stream};
use toml::Table;

use crate::bar::BarEvent;
use crate::modules::{CommonStyle, Module, ModuleData, ModuleUpdate};

#[derive(Default, Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatteryState {
    Charging,
    Discharging,
    #[default]
    Plugged,
}

#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct BatteryStatus {
    pub percentage: u8,
    pub state: BatteryState,
}

#[derive(Debug, Default)]
pub struct BatteryConfig {
    format: Box<str>,
}

#[derive(Default, Debug)]
pub struct Battery {
    status: BatteryStatus,
    config: BatteryConfig,
    style: CommonStyle,
}

impl ModuleData for BatteryStatus {}

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
}

impl Module for Battery {
    fn view(&self) -> Element<'_, BarEvent> {
        container(text(self.get_text()).color(self.style.foreground))
            .padding(self.style.padding)
            .into()
    }

    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let update = update_data
            .downcast_ref::<BatteryStatus>()
            .expect("A bug in the routing logic");
        self.status = *update;
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run(worker))
    }

    fn try_new(table: &Table) -> Option<Rc<dyn Module>>
    where
        Self: Sized,
    {
        const DEFAULT_FORMAT: &str = "{percentage}%";

        let format = table.get("format").map_or(DEFAULT_FORMAT, |format| {
            format.as_str().unwrap_or(DEFAULT_FORMAT)
        });
        let style = CommonStyle::from(table);

        let config = BatteryConfig {
            format: format.into(),
        };

        Some(Rc::new(Self {
            config,
            style,
            status: BatteryStatus::default(),
        }))
    }
}

fn value_from_file<T: FromStr>(path: impl AsRef<Path>) -> Option<T> {
    use std::fs::read_to_string;

    read_to_string(path).ok()?.trim().parse::<T>().ok()
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

fn worker() -> impl Stream<Item = ModuleUpdate> {
    const TYPE_ID: TypeId = TypeId::of::<Battery>();

    stream::channel(0, async |mut output| {
        let poll_interval = Duration::from_millis(500);

        let mut content = read_battery_info();
        output
            .send(ModuleUpdate(TYPE_ID, Arc::new(content)))
            .await
            .unwrap();
        loop {
            sleep(poll_interval);
            let new_content = read_battery_info();

            if new_content == content {
                continue;
            }

            output
                .send(ModuleUpdate(TYPE_ID, Arc::new(content)))
                .await
                .unwrap();
            content = new_content;
        }
    })
}
