mod niri;

use std::any::TypeId;
use std::rc::Rc;
use std::sync::Arc;

use iced::futures::{SinkExt, Stream};
use iced::stream;
use iced::widget::{Text, button, row, text};
use iced::{Background, Color, Subscription};

use toml::Table;

use crate::bar::Output;
use crate::logger::{warn, info};

use super::*;

use niri::NiriIpcBackend;

trait IpcBackend: Send + Sync {
    fn next_event(&mut self) -> Option<CompositorEvent>;
    // TODO: Send events
    fn try_create() -> Option<Self>
    where
        Self: Sized;
}

enum CompositorEvent {
    WorkspacesChanged(Vec<Workspace>),
    WorkspaceActivated(Workspace),
}

#[derive(Clone)]
struct Workspace {
    id: u64,
    idx: u8,
    name: Option<String>,
    is_focused: bool,
}

pub struct Workspaces {
    workspaces: Vec<Workspace>,
}

impl ModuleData for CompositorEvent {}

impl Module for Workspaces {
    fn view(&self, output: &Output) -> iced::Element<'_, crate::bar::BarEvent> {
        row(self.workspaces.iter().map(|workspace| workspace.view()))
            .spacing(2)
            .into()
    }

    fn update(&mut self, update_data: Arc<dyn ModuleData>) {
        let event = update_data
            .downcast_ref::<CompositorEvent>()
            .expect("Why is this event routed here?");

        match event {
            CompositorEvent::WorkspacesChanged(workspaces) => {
                self.workspaces.clone_from(workspaces)
            }
            CompositorEvent::WorkspaceActivated(changed_workspace) => {
                self.change_workspace(changed_workspace);
            }
        }
    }

    fn subscription(&self) -> Option<Subscription<ModuleUpdate>> {
        Some(Subscription::run(worker))
    }

    fn new_or_default(table: &Table) -> Rc<dyn Module>
    where
        Self: Sized,
    {
        let workspaces = vec![];
        // let backend = try_create_backend();

        Rc::new(Self {
            workspaces,
            // backend,
        })
    }
}

impl Workspaces {
    fn change_workspace(&mut self, new_workspace: &Workspace) {
        if !new_workspace.is_focused {
            println!("WHAT");
        }
        self.workspaces
            .iter_mut()
            .for_each(|workspace| workspace.is_focused = false);
        let Some(target_workspace) = self
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.id == new_workspace.id)
        else {
            warn("Invalid workspace ID");
            return;
        };

        target_workspace.is_focused = new_workspace.is_focused;
    }
}

impl Workspace {
    fn view(&self) -> iced::Element<'_, crate::bar::BarEvent> {
        let background = if self.is_focused {
            Background::Color(Color::from_rgb(1.0, 0., 0.))
        } else {
            Background::Color(Color::from_rgb(0., 0., 0.))
        };

        button(self.get_text())
            .style(move |_old, _arg2| button::Style {
                background: Some(background),
                text_color: Color::WHITE,
                ..Default::default()
            })
            .into()
    }

    fn get_text(&self) -> Text<'_> {
        if let Some(name) = self.name.as_ref() {
            return text(name);
        }

        text(self.idx)
    }
}

fn try_create_backend() -> Option<Box<dyn IpcBackend>> {
    if let Some(niri) = <NiriIpcBackend as IpcBackend>::try_create() {
        info("[Workspaces] using Niri backend");
        return Some(Box::new(niri));
    }

    None
}

fn worker() -> impl Stream<Item = ModuleUpdate> {
    const MODULE_ID: TypeId = TypeId::of::<Workspaces>();

    stream::channel(0, async |mut output| {
        let Some(mut ipc_backend) = try_create_backend() else {
            warn("Unsupported compositor");
            return;
        };
        loop {
            let Some(event) = ipc_backend.next_event() else {
                continue;
            };
            output
                .send(ModuleUpdate(MODULE_ID, Arc::new(event)))
                .await
                .expect("Broken pipe");
        }
    })
}
