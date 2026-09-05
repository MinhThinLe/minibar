use std::io::{BufRead, BufReader};
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use iced::Background;
use iced::widget::{container, text};
use iced::{Element, Subscription, futures::Stream, stream};
use log::error;
use minibar_derives::{ModuleData, NamedModule};
use toml::Table;

use crate::modules::module_id::module_id_unique;

use super::*;

const DEFAULT_FORMAT: &str = "VOL: {volume_level}";
const DEFAULT_FORMAT_MUTED: &str = "MUTED";

#[derive(ModuleData)]
enum PipeWireUpdate {
    Volume(Volume),
}

#[derive(Clone, Copy)]
struct Volume {
    level: u16,
    is_muted: bool,
}

struct PipeWireConfig {
    format: Box<str>,
    format_muted: Box<str>,
    icons: Vec<char>,
    icon_muted: Box<str>,
}

#[derive(NamedModule)]
pub struct PipeWire {
    volume: Volume,
    config: PipeWireConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for PipeWire {
    fn update(&mut self, module_update: ModuleUpdate) {
        let ModuleUpdate(_module_id, update_data) = module_update;
        let update = update_data
            .downcast_ref::<PipeWireUpdate>()
            .expect("Bro thought he was on the team");
        match update {
            PipeWireUpdate::Volume(volume) => self.volume = *volume,
        }
    }

    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        container(text(self.get_text()))
            .style(|_theme| container::Style {
                text_color: self.style.foreground,
                background: self.get_background(),
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

    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT).into();
        let format_muted = get_str(table, "format_muted")
            .unwrap_or(DEFAULT_FORMAT_MUTED)
            .into();

        let icons = to_icon_list(get_str(table, "icons").unwrap_or_default());
        let icon_muted = get_str(table, "icon_muted").unwrap_or_default().into();

        let config = PipeWireConfig {
            format,
            format_muted,
            icons,
            icon_muted,
        };
        let style = CommonStyle::from(table);

        let id = [module_id_unique()];

        Box::new(Self {
            volume: get_new_volume(),
            config,
            style,
            id,
        })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
    }
}

impl PipeWire {
    fn get_text(&self) -> String {
        if self.volume.is_muted {
            return self.format(&self.config.format_muted);
        }

        self.format(&self.config.format)
    }

    fn format(&self, format_str: &str) -> String {
        format_str
            .replace("{icon}", &self.get_icon().to_string())
            .replace("{icon_muted}", &self.config.icon_muted)
            .replace("{volume}", &self.volume.level.to_string())
    }

    fn get_icon(&self) -> char {
        const ASSUMED_MAX_VOLUME: u16 = 100;
        if self.config.icons.is_empty() {
            return char::default();
        }

        let levels = self.config.icons.len() as u16;
        let step = ASSUMED_MAX_VOLUME / levels;

        for level in 1..levels {
            if self.volume.level < level * step {
                return self.config.icons[level as usize - 1];
            }
        }

        self.config.icons.last().copied().unwrap_or_default()
    }

    fn get_background(&self) -> Option<Background> {
        Some(Background::Color(self.style.background?))
    }
}

fn to_icon_list(icons_str: &str) -> Vec<char> {
    icons_str
        .chars()
        .filter(|char| !char.is_whitespace())
        .collect()
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    /*
     * Fuck this pure rust binding shits, I just wanted to get the default sink's current volume,
     * not perform some gymnastic. This module gets the current module by waiting `pw-mon` for
     * changes and read `wpctl get-volume @DEFAULT_AUDIO_SINK` when changes do happen. This is
     * insanely hacky. Deal with it.
     */
    stream::channel(0, async move |mut output| {
        let (sender, receiver) = mpsc::channel();

        std::thread::spawn(move || {
            let Ok(change_watcher) = Command::new("pw-mon").stdout(Stdio::piped()).spawn() else {
                error!("Failed to start `pw-mon`, is it available in $PATH?");
                return;
            };

            let Some(child_stdout) = change_watcher.stdout else {
                error!("Failed to pipe `pw-mon`'s child process to the main thread'");
                return;
            };

            let mut reader = BufReader::new(child_stdout);
            let mut buffer = String::new();

            loop {
                buffer.clear();
                let _bytes = reader.read_line(&mut buffer).expect("Child process died?");
                sender
                    .send(())
                    .expect("Could not send data between channels");
            }
        });

        loop {
            const MAXIMUM_CHANNEL_LATENCY: Duration = Duration::from_millis(10);
            match receiver.recv_timeout(MAXIMUM_CHANNEL_LATENCY) {
                Ok(()) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    /*
                     * Remeber what I said about how janky this is?, turns out, calling `wpctl`
                     * also triggers `pw-mon` and thus, we need to delay calling `get_new_volume()`
                     * by one event. The rationale could be more intuitively explained by the
                     * diagram below.
                     * This is how things would behave if I were to put `reveicer.recv()` at the
                     * very end of the function
                     * pw-mon wakes up -> wait for termination -> get_new_volume -> wait for new events
                     *  ^                                              |
                     *  |______________________________________________|
                     *
                     * So to prevent spam, I have to structure things like this
                     *
                     * pw-mon wakes up -> wait for termination -> wait for new events -> get_new_volume
                     *  ^                                                                     |
                     *  |_______(still happens but filtered by the timeout requirement)_______|
                     *
                     * This way, get_new_volume would still wake up pw-mon but its output would be
                     * batched with what it already sends.
                     */
                    let _ = receiver.recv();
                    let event = PipeWireUpdate::Volume(get_new_volume());
                    send_data(&mut output, module_id, event).await;
                }
                _ => return,
            }
        }
    })
}

fn get_new_volume() -> Volume {
    let output = Command::new("wpctl")
        .arg("get-volume")
        .arg("@DEFAULT_AUDIO_SINK@")
        .output()
        .expect("Could not start `wpctl`, is it available in your $PATH?");

    let new_volume = String::from_utf8_lossy(&output.stdout);
    let mut fields = new_volume.split_whitespace();

    // Expected format: Volume: 0.80 [MUTED]
    let volume = fields.nth(1).map_or(0, |str| {
        (str.parse::<f32>().unwrap_or_default() * 100.0) as u16
    });
    let is_muted = fields.next().is_some();

    Volume {
        level: volume,
        is_muted,
    }
}
