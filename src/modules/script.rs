use std::process::Command;
use std::sync::Arc;
use std::thread::sleep;

use super::prelude::*;

#[derive(ModuleData)]
struct CommandOutput(String);

#[derive(Clone)]
struct ScriptConfig {
    command: Arc<str>,
    poll_interval: Duration,
}

pub struct Script {
    config: ScriptConfig,
    output_buffer: String,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for Script {
    fn id(&self) -> &[ModuleId] {
        &self.id
    }

    fn update(&mut self, module_update: ModuleUpdate) {
        let ModuleUpdate(_module_id, update_data) = module_update;
        let command_ouput = update_data
            .downcast_ref::<CommandOutput>()
            .expect("Who sent me this?");

        self.output_buffer.clone_from(&command_ouput.0);
    }

    fn view(&self, _output: &Output) -> Element<'_, BarEvent> {
        container(text(&self.output_buffer).wrapping(text::Wrapping::None))
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
        Some(Subscription::run_with(
            (
                self.id[0],
                self.config.command.clone(),
                self.config.poll_interval,
            ),
            move |(destination, command, poll_interval)| {
                worker(*destination, *poll_interval, command.clone())
            },
        ))
    }

    fn new_or_default(table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        let id = [module_id_unique()];

        let poll_interval = get_duration(table, "poll_interval").unwrap_or(DEFAULT_POLL_INTERVAL);
        let command = get_str(table, "command").unwrap_or_default().into();

        let config = ScriptConfig {
            command,
            poll_interval,
        };

        let style = CommonStyle::from(table);

        Box::new(Self {
            id,
            config,
            style,
            output_buffer: String::new(),
        })
    }
}

fn worker(
    destination: ModuleId,
    poll_interval: Duration,
    command: Arc<str>,
) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let mut subprocess = Command::new("sh");
        let command = subprocess.arg("-c").arg(command.as_ref());

        loop {
            let Some(result) = unwrap_output(command.output()) else {
                return;
            };

            let content = CommandOutput(result);
            send_data(&mut output, destination, content).await;

            sleep(poll_interval);
        }
    })
}

fn unwrap_output(command_output: std::io::Result<std::process::Output>) -> Option<String> {
    let result = match command_output {
        Ok(result) => result,
        Err(err) => {
            error!("{err}");
            return None;
        }
    };

    if !result.status.success() {
        error!("{}", String::from_utf8_lossy(&result.stderr).trim());
        return None;
    }

    match String::from_utf8(result.stdout) {
        Ok(mut res) => {
            // Premature optimisation moment
            res.truncate(res.trim_end().len());
            Some(res)
        }
        Err(utf8_err) => {
            error!("{utf8_err}");
            None
        }
    }
}
