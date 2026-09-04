use std::{process::Command, thread::sleep};

use iced::{futures::Stream, stream, widget::text};
use log::error;
use minibar_derives::ModuleData;

use crate::modules::module_id::module_id_unique;

use super::*;

#[derive(ModuleData)]
struct CommandOutput(String);

struct ScriptConfig {
    command: Arc<str>,
    name: Arc<str>,
}

pub struct Script {
    config: ScriptConfig,
    output_buffer: String,
    id: [ModuleId; 1],
}

impl Module for Script {
    fn id(&self) -> &[ModuleId] {
        &self.id
    }

    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let command_ouput = update_data
            .downcast_ref::<CommandOutput>()
            .expect("Who sent me this?");

        self.output_buffer.clone_from(&command_ouput.0);
    }

    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        text!("{}", self.output_buffer.trim()).into()
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run_with(
            (
                self.id[0],
                self.config.command.clone(),
                self.config.name.clone(),
            ),
            move |(destination, command, name)| worker(*destination, command.clone(), name.clone()),
        ))
    }

    fn new_or_default(table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        let id = [module_id_unique()];

        let config = ScriptConfig {
            name: "custom-module".into(),
            command: "date".into(),
        };

        Rc::new(Self {
            id,
            config,
            output_buffer: String::new(),
        })
    }
}

fn worker(
    destination: ModuleId,
    command: Arc<str>,
    module_name: Arc<str>,
) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let poll_interval = get_poll_interval(&module_name);

        let mut subprocess = Command::new("sh");
        let command = subprocess.arg("-c").arg(command.as_ref());
        loop {
            let result = match command.output() {
                Ok(result) => String::from_utf8(result.stdout),
                Err(err) => {
                    error!("{err}");
                    return;
                }
            };
            let content = match result {
                Ok(result) => CommandOutput(result),
                Err(utf8_err) => {
                    error!("{utf8_err}");
                    return;
                }
            };
            send_data(&mut output, destination, content).await;

            sleep(poll_interval);
        }
    })
}
