use std::{
    any::TypeId,
    fs::File,
    io::{BufRead, BufReader},
    rc::Rc,
    str::FromStr,
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use iced::{
    Subscription,
    futures::{SinkExt, Stream},
    stream,
    widget::text,
};

use crate::modules::{Module, ModuleData, ModuleUpdate};

#[derive(Debug, Clone)]
struct CoreStat {
    // These fields can never be negative but it makes subtractions hell of a lot easier
    all: i64,
    // user: i64,
    // system: i64,
    // guest: i64,
    total: i64,
}

#[derive(Debug, Clone)]
struct CoresStat(Vec<CoreStat>);

#[derive(Debug, Clone, Copy)]
enum CoreId {
    All,
    Core(u8),
}

#[derive(Debug)]
pub struct Cpu {
    current_core_stats: CoresStat,
    last_core_stats: CoresStat,
}

impl ModuleData for CoresStat {}

impl Module for Cpu {
    fn view(&self) -> iced::Element<'_, crate::bar::BarEvent> {
        text!("CPU: {:.1}%", self.measure_core_load(CoreId::All)).into()
    }

    fn update(&mut self, update_data: std::sync::Arc<dyn super::ModuleData>) {
        let cores = update_data
            .downcast_ref::<CoresStat>()
            .expect("A bug in the routing logic");

        self.last_core_stats = self.current_core_stats.clone();
        self.current_core_stats = cores.clone();
    }

    fn subscription(&self) -> Option<iced::Subscription<super::ModuleUpdate>> {
        Some(Subscription::run(worker))
    }

    fn new_or_default(table: &toml::Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        // TODO: Add configuration options for this module
        Rc::new(Self {
            current_core_stats: CoresStat(Vec::new()),
            last_core_stats: CoresStat(Vec::new()),
        })
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
    fn measure_core_load(&self, core_id: CoreId) -> f32 {
        let index = core_id.to_index();
        if self
            .current_core_stats
            .0
            .len()
            .min(self.last_core_stats.0.len())
            <= index
        {
            return 0.0;
        }

        let current_stat = &self.current_core_stats.0[core_id.to_index()];
        let last_stat = &self.last_core_stats.0[core_id.to_index()];

        // Not enough information to deduce the core's load from
        if current_stat.total == last_stat.total {
            return 0.0;
        }

        let delta_total = current_stat.total - last_stat.total;
        let delta_all = current_stat.all - last_stat.all;

        (delta_all as f32 / delta_total as f32) * 100.0
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

fn worker() -> impl Stream<Item = ModuleUpdate> {
    const MODULE_ID: TypeId = TypeId::of::<Cpu>();

    stream::channel(0, async |mut output| {
        let poll_interval = Duration::from_secs(1);

        output
            .send(ModuleUpdate(MODULE_ID, Arc::new(measure())))
            .await
            .expect("Broken pipe");

        loop {
            sleep(poll_interval);

            output
                .send(ModuleUpdate(MODULE_ID, Arc::new(measure())))
                .await
                .expect("Broken pipe");
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
