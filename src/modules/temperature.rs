use std::{fs, thread::sleep};

use iced::{
    Background, Color,
    futures::Stream,
    stream,
    widget::{container, text},
};

use log::error;
use minibar_derives::{ModuleData, NamedModule};

use crate::modules::module_id::module_id_unique;

use super::*;

const DEFAULT_FORMAT: &str = "TEMP: {temp_c}°C";
const DEFAULT_CRITICAL_THRESHOLD: f32 = 80.0;

#[derive(ModuleData, Clone, Copy)]
struct TemperatureReading(f32);

struct TemperatureConfig {
    format: Box<str>,
    critical_threshold: f32,
    critical_foreground: Option<Color>,
}

#[derive(NamedModule)]
pub struct Temperature {
    config: TemperatureConfig,
    style: CommonStyle,
    current_temp: TemperatureReading,
    id: [ModuleId; 1], // Looks kinda stupid but this should avoid an allocation
}

impl Module for Temperature {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        container(text(self.get_text()))
            .padding(self.style.padding)
            .style(|_theme| container::Style {
                text_color: self.get_color(),
                background: self.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let new_data = update_data
            .downcast_ref::<TemperatureReading>()
            .expect("This update shouldn't be here");

        self.current_temp = *new_data;
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run_with(self.id[0], |destination| {
            worker(*destination)
        }))
    }

    fn new_or_default(table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();

        let critical_threshold =
            get_float(table, "critical_threshold").unwrap_or(DEFAULT_CRITICAL_THRESHOLD);
        let critical_foreground = get_color(table, "critical_foreground");

        let style = CommonStyle::from(table);

        let config = TemperatureConfig {
            format,
            critical_threshold,
            critical_foreground,
        };

        let id = [module_id_unique()];

        Rc::new(Self {
            config,
            style,
            current_temp: TemperatureReading(0.0),
            id,
        })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }
}

impl Temperature {
    fn get_text(&self) -> String {
        const TEMP_C: &str = "{temp_c}";
        const TEMP_F: &str = "{temp_f}";
        const TEMP_K: &str = "{temp_k}";
        self.config
            .format
            .replace(TEMP_C, &float_to_string(self.current_temp.0))
            .replace(TEMP_F, &float_to_string(self.current_temp.to_fahrenheit()))
            .replace(TEMP_K, &float_to_string(self.current_temp.to_kelvin()))
    }

    fn get_color(&self) -> Option<Color> {
        if self.current_temp.0 > self.config.critical_threshold {
            return self.config.critical_foreground;
        }
        self.style.foreground
    }

    fn get_background(&self) -> Option<Background> {
        Some(Background::Color(self.style.background?))
    }
}

impl TemperatureReading {
    fn to_fahrenheit(self) -> f32 {
        self.0 * 1.8 + 32.0
    }

    fn to_kelvin(self) -> f32 {
        self.0 + 273.15
    }
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let poll_interval = get_poll_interval("temperature");
        let reading = TemperatureReading(read_temp());

        send_data(&mut output, module_id, reading).await;

        loop {
            sleep(poll_interval);
            let reading = TemperatureReading(read_temp());

            send_data(&mut output, module_id, reading).await;
        }
    })
}

fn read_temp() -> f32 {
    const THERMAL_DIR: &str = "/sys/class/thermal/";
    const THERMAL_ZONE_DIR: &str = "/sys/class/thermal/thermal_zone";

    const TYPE_DIR: &str = "type";
    const DESIRED_SENSOR_TYPE: &str = "SEN";
    const TEMPERATURE: &str = "temp";

    const MILICELCIUS: f32 = 0.001;

    let Ok(thermal_dirs) = fs::read_dir(THERMAL_DIR) else {
        error!("[Temperature] Could not read {THERMAL_DIR}, no meaningful data will be polled");
        return 0.0;
    };

    let thermal_dirs: Vec<_> = thermal_dirs
        .filter_map(|dir| dir.ok().map(|dir| dir.path()))
        .filter(|path| path.to_string_lossy().starts_with(THERMAL_ZONE_DIR))
        .collect();

    let mut average_temp: f32 = 0.0;
    let mut entries = 0;
    for dir in &thermal_dirs {
        let Ok(thermal_type) = fs::read_to_string(dir.join(TYPE_DIR)) else {
            continue;
        };
        if !thermal_type.starts_with(DESIRED_SENSOR_TYPE) {
            continue;
        }
        let Some(temp) = value_from_file::<f32>(dir.join(TEMPERATURE)) else {
            continue;
        };

        average_temp += temp;
        entries += 1;
    }

    average_temp * MILICELCIUS / entries as f32
}
