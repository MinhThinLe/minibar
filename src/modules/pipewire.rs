use std::any::TypeId;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{process::Command, rc::Rc};

use iced::widget::text;
use iced::{Element, Subscription, futures::Stream, stream};
use log::error;
use minibar_derives::{ModuleData, NamedModule};
use toml::Table;

use crate::modules::send_data;

use super::{BarEvent, Module, ModuleData, ModuleUpdate, NamedModule, Output};

#[derive(ModuleData)]
enum PipeWireUpdate {
    Volume(Volume),
}

#[derive(Clone, Copy)]
struct Volume {
    level: u16,
    is_muted: bool,
}

#[derive(NamedModule)]
pub struct PipeWire {
    volume: Volume,
}

impl Module for PipeWire {
    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let update = update_data
            .downcast_ref::<PipeWireUpdate>()
            .expect("Bro thought he was on the team");
        match update {
            PipeWireUpdate::Volume(volume) => self.volume = *volume,
        }
    }
    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        text!(
            "Volume: {}, is muted: {}",
            self.volume.level,
            self.volume.is_muted
        )
        .into()
    }
    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run(worker))
    }
    fn new_or_default(_table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        Rc::new(Self {
            volume: get_new_volume(),
        })
    }
}

fn worker() -> impl Stream<Item = ModuleUpdate> {
    /*
     * Fuck this pure rust binding shits, I just wanted to get the default sink's current volume,
     * not perform some gymnastic. This module gets the current module by waiting `pw-mon` for
     * changes and read `wpctl get-volume @DEFAULT_AUDIO_SINK` when changes do happen. This is
     * insanely hacky. Deal with it.
     */
    const PIPEWIRE_MODULE_ID: TypeId = TypeId::of::<PipeWire>();

    stream::channel(0, async |mut output| {
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
                    .send(_bytes)
                    .expect("Could not send data between channels");
            }
        });

        loop {
            const MAXIMUM_CHANNEL_LATENCY: Duration = Duration::from_millis(10);
            match receiver.recv_timeout(MAXIMUM_CHANNEL_LATENCY) {
                Ok(_) => {}
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
                    send_data(&mut output, PIPEWIRE_MODULE_ID, event).await;
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
