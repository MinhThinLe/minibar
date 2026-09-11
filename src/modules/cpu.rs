use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::thread::sleep;

use super::prelude::*;

const DEFAULT_FORMAT: &str = "CPU: {usage}";
const DEFAULT_CRITICAL_THRESHOLD: u8 = 100;

#[derive(Clone)]
struct CoreStat {
    // These fields can never be negative but it makes subtractions hell of a lot easier
    all: i64,
    // user: i64,
    // system: i64,
    // guest: i64,
    total: i64,
}

#[derive(Clone, ModuleData)]
struct CoresStat(Vec<CoreStat>);

#[derive(Debug, Clone, Copy)]
enum CoreId {
    All,
    #[allow(dead_code)] // For when I decide to implement tooltips later on
    Core(u8),
}

struct CpuConfig {
    format: Box<str>,
    poll_interval: Duration,
    critical_threshold: u8,
    critical_foreground: Option<Color>,
}

#[derive(NamedModule)]
pub struct Cpu {
    current_core_stats: CoresStat,
    last_core_stats: CoresStat,
    config: CpuConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for Cpu {
    fn view(&self, _output: &Output) -> iced::Element<'_, crate::bar::BarEvent> {
        container(text(self.get_text()))
            .padding(self.style.padding)
            .style(|_old_style| container::Style {
                text_color: self.get_color(),
                background: self.style.get_background(),
                border: self.style.border,
                ..Default::default()
            })
            .into()
    }

    fn update(&mut self, module_update: ModuleUpdate) {
        let ModuleUpdate(_module_id, update_data) = module_update;
        let cores = update_data
            .downcast_ref::<CoresStat>()
            .expect("A bug in the routing logic");

        self.last_core_stats = self.current_core_stats.clone();
        self.current_core_stats = cores.clone();
    }

    fn subscription(&self) -> Option<iced::Subscription<ModuleUpdate>> {
        Some(Subscription::run_with(
            (self.id[0], self.config.poll_interval),
            |(destination, poll_interval)| worker(*destination, *poll_interval),
        ))
    }

    fn new_or_default(table: &toml::Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        // TODO: Add configuration options for this module
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();

        let critical_threshold = get_int(table, "threshold").unwrap_or(DEFAULT_CRITICAL_THRESHOLD);
        let critical_foreground = get_color(table, "critical_foreground");
        let poll_interval = get_duration(table, "poll_interval").unwrap_or(DEFAULT_POLL_INTERVAL);

        let style = CommonStyle::from(table);

        let config = CpuConfig {
            format,
            poll_interval,
            critical_threshold,
            critical_foreground,
        };

        let id = [module_id_unique()];

        Box::new(Self {
            config,
            style,
            current_core_stats: CoresStat(vec![]),
            last_core_stats: CoresStat(vec![]),
            id,
        })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }
}

impl CoreId {
    fn to_index(self) -> usize {
        match self {
            CoreId::All => 0,
            CoreId::Core(core_id) => core_id as usize + 1,
        }
    }
}

impl Cpu {
    fn measure_core_load(&self, core_id: CoreId) -> u8 {
        let index = core_id.to_index();
        if self
            .current_core_stats
            .0
            .len()
            .min(self.last_core_stats.0.len())
            <= index
        {
            return 0;
        }

        let current_stat = &self.current_core_stats.0[core_id.to_index()];
        let last_stat = &self.last_core_stats.0[core_id.to_index()];

        // Not enough information to deduce the core's load from
        if current_stat.total == last_stat.total {
            return 0;
        }

        let delta_total = current_stat.total - last_stat.total;
        let delta_all = current_stat.all - last_stat.all;
        let usage = (delta_all as f32 / delta_total as f32) * 100.0;
        usage.round() as u8
    }

    fn get_color(&self) -> Option<Color> {
        let foreground = self.style.foreground?;
        if self.measure_core_load(CoreId::All) < self.config.critical_threshold {
            return Some(foreground);
        }
        self.config.critical_foreground
    }

    fn get_text(&self) -> String {
        self.config
            .format
            .replace("{usage}", &self.measure_core_load(CoreId::All).to_string())
    }
}

impl FromStr for CoreStat {
    type Err = ();
    // Expected format:
    // id (unused)   user    nice system idle     iowait irq    softirq steal guest guest_nice
    // cpu0          2165897 135  468821 51205466 70701  160469 77884   0     0     0
    // Note: The values are separated from eachother by only a single space
    fn from_str(values: &str) -> Result<Self, Self::Err> {
        let values: Vec<i64> = values
            .split_whitespace()
            .filter_map(|item| item.parse().ok())
            .collect();

        let [
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            _guest,
            _guest_nice,
        ] = values[..]
        else {
            // This shouldn't fail (unless the linux kernel adds another field to /proc/stat)
            return Err(());
        };

        let all = user + nice + system + irq + softirq;
        // TODO: find a use for these fields later
        // let user = user + nice;
        // let guest = guest + guest_nice;
        let total = all + idle + iowait + steal;

        Ok(CoreStat {
            all,
            // user,
            // system,
            // guest,
            total,
        })
    }
}

fn measure() -> CoresStat {
    let file = File::open("/proc/stat").expect("No /proc/stat?");
    let reader = BufReader::new(file);
    let core_stats: Vec<CoreStat> = reader
        .lines()
        .filter_map(|line| {
            line.ok().and_then(|line| {
                line.split_once(' ')
                    .and_then(|(_cpu_id, stats)| CoreStat::from_str(stats).ok())
            })
        })
        .collect();

    CoresStat(core_stats)
}

fn worker(module_id: ModuleId, poll_interval: Duration) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        loop {
            send_data(&mut output, module_id, measure()).await;
            sleep(poll_interval);
        }
    })
}

#[test]
fn test_cpu_measure() {
    use std::fs;

    let core_stats = measure();
    let core_count = fs::read_to_string("/proc/stat")
        .unwrap()
        .lines()
        .collect::<Vec<&str>>()
        .len()
        - 7;
    assert_eq!(core_stats.0.len(), core_count);
}
