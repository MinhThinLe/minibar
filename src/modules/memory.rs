use std::fmt::Display;
use std::fs;
use std::thread::sleep;

use super::*;

const DEFAULT_FORMAT: &str = "RAM: {usage}";
const DEFAULT_CRITICAL_THRESHOLD: u8 = 80;

#[allow(clippy::enum_variant_names)]
enum MemorySize {
    KiloBytes(f64),
    MegaBytes(f64),
    GigaBytes(f64),
    TeraBytes(f64),
    // Higher memory capacity? In this economy?
}

#[derive(Default, ModuleData, Clone, Copy)]
struct MemoryStatus {
    // All in kB
    total: u64,
    free: u64,
    available: u64,
    buffers: u64,
    cached: u64,
}

struct MemoryConfig {
    format: Box<str>,
    critical_threshold: u8,
    critical_foreground: Option<Color>,
}

#[derive(NamedModule)]
pub struct Memory {
    status: MemoryStatus,
    config: MemoryConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for Memory {
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        container(text(self.get_text()))
            .padding(self.style.padding)
            .style(|_theme| container::Style {
                text_color: self.get_color(),
                background: self.style.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, module_update: ModuleUpdate) {
        let ModuleUpdate(_module_id, update_data) = module_update;
        let mem_info = update_data
            .downcast_ref::<MemoryStatus>()
            .expect("This shouldn't end up here");

        self.status = *mem_info;
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
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();

        let critical_threshold = get_int(table, "critical_threshold")
            .map_or(DEFAULT_CRITICAL_THRESHOLD, |threshold| threshold as u8);
        let critical_foreground = get_color(table, "critical_forground");

        let style = CommonStyle::from(table);

        let config = MemoryConfig {
            format,
            critical_threshold,
            critical_foreground,
        };

        let status = MemoryStatus::default();

        let id = [module_id_unique()];

        Box::new(Self {
            status,
            config,
            style,
            id,
        })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }
}

impl FromStr for MemoryStatus {
    type Err = ();
    // Expected format
    // MemTotal:       16059872 kB
    // MemFree:         5773216 kB
    // MemAvailable:   10290124 kB
    // Buffers:          300296 kB
    // Cached:          4533656 kB
    // --- Truncated for clarity ---
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const FIELDS: [&str; 5] = [
            // These entries have to be in the same order as /proc/meminfo
            "MemTotal:",
            "MemFree:",
            "MemAvailable:",
            "Buffers:",
            "Cached:",
        ];
        let mut fields = Vec::with_capacity(FIELDS.len());
        for (index, line) in s.lines().enumerate() {
            let Some(field_name) = FIELDS.get(index) else {
                break;
            };
            let line_length = line.len() - " kB".len();
            fields.push(
                line[field_name.len()..line_length]
                    .trim_start()
                    .parse()
                    .expect("WHAT?"),
            );
        }

        let [total, free, available, buffers, cached, ..] = fields[..] else {
            return Err(());
        };

        Ok(MemoryStatus {
            total,
            free,
            available,
            buffers,
            cached,
        })
    }
}

impl Memory {
    fn get_text(&self) -> String {
        const USAGE: &str = "{usage}";
        const USAGE_PERCENTAGE: &str = "{percent_used}";

        self.config
            .format
            .replace(USAGE, &self.status.usage().to_string())
            .replace(USAGE_PERCENTAGE, &self.status.percent_used().to_string())
    }

    fn get_color(&self) -> Option<Color> {
        if self.status.percent_used() > self.config.critical_threshold {
            return self.config.critical_foreground;
        }

        self.style.foreground
    }
}

impl Display for MemorySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(!self.can_promote(), "Fully promote this enum first");
        write!(f, "{:.1} {}", self.inner_value(), self.unit())
    }
}

impl MemoryStatus {
    fn usage(&self) -> MemorySize {
        MemorySize::new(self.total - self.free - self.buffers - self.cached)
    }

    fn percent_used(&self) -> u8 {
        (100.0 - (self.available as f32 / self.total as f32) * 100.0) as u8
    }
}

impl MemorySize {
    fn new(mem_kb: u64) -> Self {
        let mut size = MemorySize::KiloBytes(mem_kb as f64);
        while size.can_promote() {
            size = size.promote();
        }

        size
    }

    fn inner_value(&self) -> f64 {
        match self {
            MemorySize::KiloBytes(size)
            | MemorySize::MegaBytes(size)
            | MemorySize::GigaBytes(size)
            | MemorySize::TeraBytes(size) => *size,
        }
    }

    fn can_promote(&self) -> bool {
        if let MemorySize::TeraBytes(_) = self {
            return false;
        }
        self.inner_value() > 1000.0
    }

    fn promote(self) -> Self {
        match self {
            Self::KiloBytes(value) => Self::MegaBytes(value / 1000.0),
            Self::MegaBytes(value) => Self::GigaBytes(value / 1000.0),
            Self::GigaBytes(value) => Self::TeraBytes(value / 1000.0),
            Self::TeraBytes(value) => Self::TeraBytes(value), // This guy has RAM, get him
        }
    }

    fn unit(&self) -> &'static str {
        match self {
            Self::KiloBytes(_) => "KB",
            Self::MegaBytes(_) => "MB",
            Self::GigaBytes(_) => "GB",
            Self::TeraBytes(_) => "TB",
        }
    }
}

fn measure() -> MemoryStatus {
    let mem_details =
        fs::read_to_string("/proc/meminfo").expect("Unable to read from /proc/meminfo");
    MemoryStatus::from_str(&mem_details).unwrap_or_default()
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let poll_interval = get_poll_interval("memory");

        loop {
            send_data(&mut output, module_id, measure()).await;
            sleep(poll_interval);
        }
    })
}
