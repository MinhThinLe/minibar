use std::fs;
use std::path::PathBuf;
use std::thread::sleep;

use super::*;

const DEFAULT_FORMAT: &str = "LIGHT: {percentage}%";

#[derive(Debug)]
struct BacklightReaderFs {
    path: PathBuf,
    last_value: u32,
}

#[derive(ModuleData, Clone)]
struct BacklightStatus {
    display: String,
    max_brightness: u32,
    current_brightness: u32,
}

struct BacklightConfig {
    format: Box<str>,
    icons: Vec<char>,
}

#[derive(NamedModule)]
pub struct Backlight {
    statuses: Vec<BacklightStatus>,
    config: BacklightConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for Backlight {
    fn update(&mut self, module_update: ModuleUpdate) {
        let update = module_update
            .1
            .downcast_ref::<BacklightStatus>()
            .expect("Who sent me this?");

        let Some(backlight) = self
            .statuses
            .iter_mut()
            .find(|item| item.display == update.display)
        else {
            self.statuses.push(update.clone());
            return;
        };

        backlight.current_brightness = update.current_brightness;
    }

    fn view(&self, output: &Output) -> Element<'_, BarEvent> {
        container(text(self.get_text_for_output(output)))
            .padding(self.style.padding)
            .style(|_theme| container::Style {
                text_color: self.style.foreground,
                background: self.style.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run_with(self.id[0], |destination| {
            worker(*destination)
        }))
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }

    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        let id = [module_id_unique()];

        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();
        let icons = to_icon_list(get_str(table, "icons").unwrap_or_default());

        let config = BacklightConfig { format, icons };

        let style = CommonStyle::from(table);

        Box::new(Self {
            statuses: Vec::new(),
            config,
            style,
            id,
        })
    }
}

impl BacklightReaderFs {
    const MAX_BRIGHTNESS_FILE: &str = "max_brightness";
    const CURRENT_BRIGHTNESS_FILE: &str = "brightness";
    const DEVICE_PATH: &str = "device";

    fn get_all() -> Vec<Self> {
        const BACKLIGHT_DIRECTORY: &str = "/sys/class/backlight";
        let Ok(backlight_paths) = fs::read_dir(BACKLIGHT_DIRECTORY) else {
            error!(
                "No backlight provider, please check /sys/class/backlight and verify that it isn't empty"
            );
            return Vec::new();
        };
        backlight_paths
            .filter_map(|path| {
                path.ok().map(|dir| Self {
                    path: dir.path(),
                    last_value: 0,
                })
            })
            .collect()
    }

    fn get_max_brightness(&self) -> u32 {
        let path = self.path.join(Self::MAX_BRIGHTNESS_FILE);
        value_from_file(path).unwrap_or_default()
    }

    fn get_current_brightness(&self) -> u32 {
        let path = self.path.join(Self::CURRENT_BRIGHTNESS_FILE);
        value_from_file(path).unwrap_or_default()
    }

    fn get_display(&self) -> Option<String> {
        let path = self.path.join(Self::DEVICE_PATH);
        let device_path = fs::read_link(path).ok()?;
        let device_name = device_path.file_name()?.to_string_lossy();
        Some(device_name.split_once('-')?.1.to_string())
    }

    fn read(&self) -> BacklightStatus {
        let current_brightness = self.get_current_brightness();
        let max_brightness = self.get_max_brightness();
        let display = self.get_display().unwrap_or_default();

        BacklightStatus {
            display,
            max_brightness,
            current_brightness,
        }
    }
}

impl Backlight {
    fn get_text_for_output(&self, output: &Output) -> String {
        const PERCENTAGE: &str = "{percentage}";
        const ICON: &str = "{icon}";
        let output_name = output_name(output).unwrap_or_default();
        let Some(status) = self
            .statuses
            .iter()
            .find(|output| output.display == output_name)
        else {
            return String::new();
        };

        let percentage = status.percent();
        let icon = get_icon(&self.config.icons, percentage as u16, 100);

        self.config
            .format
            .replace(PERCENTAGE, &percentage.to_string())
            .replace(ICON, &icon.to_string())
    }
}

impl BacklightStatus {
    fn percent(&self) -> u32 {
        (self.current_brightness * 100) / self.max_brightness
    }
}

fn output_name(output: &Output) -> Option<String> {
    let Some(output_info) = &output.info else {
        return None;
    };
    output_info.name.clone()
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let mut backlight_readers = BacklightReaderFs::get_all();
        let poll_interval = get_poll_interval("backlight");
        loop {
            for reader in &mut backlight_readers {
                let status = reader.read();
                if status.current_brightness == reader.last_value {
                    continue;
                }
                reader.last_value = status.current_brightness;
                send_data(&mut output, module_id, status).await;
            }
            sleep(poll_interval);
        }
    })
}
