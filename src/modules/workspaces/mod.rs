mod niri;

use iced::widget::{Text, button};

use super::*;

use niri::NiriIpcBackend;

const DEFAULT_FORMAT: &str = "{index}";
const DEFAULT_FOCUSED_COLOR: Color = Color::from_rgb8(255, 0, 0);
const DEFAULT_SPACING: u16 = 2;

trait IpcBackend: Send + Sync {
    fn next_event(&mut self) -> Option<CompositorEvent>;
    // TODO: Send events
    fn try_create() -> Option<Self>
    where
        Self: Sized;
}

#[derive(ModuleData)]
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
    output: String,
}

struct WorkspacesConfig {
    format: Box<str>,
    focused_color: Color,
    spacing: u16,
}

#[derive(NamedModule)]
pub struct Workspaces {
    #[allow(clippy::struct_field_names)]
    workspaces: Vec<Workspace>,
    config: WorkspacesConfig,
    style: CommonStyle,
    id: [ModuleId; 1],
}

impl Module for Workspaces {
    fn view(&self, output: &Output) -> iced::Element<'_, crate::bar::BarEvent> {
        let output_name = || -> Option<&str> {
            let output_info = output.info.as_ref()?;
            output_info.name.as_deref()
        }()
        .unwrap_or_default();

        let workspaces = self
            .workspaces
            .iter()
            .filter(|workspace| workspace.output == output_name)
            .map(|workspace| workspace.view(&self.style, &self.config));

        row(workspaces)
            .spacing(self.config.spacing)
            .padding(self.style.padding)
            .into()
    }

    fn update(&mut self, module_update: ModuleUpdate) {
        let ModuleUpdate(_module_id, update_data) = module_update;
        let event = update_data
            .downcast_ref::<CompositorEvent>()
            .expect("Why is this event routed here?");

        match event {
            CompositorEvent::WorkspacesChanged(workspaces) => {
                self.workspaces.clone_from(workspaces);
            }
            CompositorEvent::WorkspaceActivated(changed_workspace) => {
                self.change_workspace(changed_workspace);
            }
        }
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
        let format = get_str(table, "format").unwrap_or(DEFAULT_FORMAT);
        let focused_color = get_color(table, "focused_color").unwrap_or(DEFAULT_FOCUSED_COLOR);
        let spacing = get_int(table, "spacing").map_or(DEFAULT_SPACING, |int| int as u16);

        let style = CommonStyle::from(table);

        let config = WorkspacesConfig {
            format: format.into(),
            focused_color,
            spacing,
        };

        let id = [module_id_unique()];

        Box::new(Self {
            workspaces: Vec::new(),
            config,
            style,
            id,
        })
    }

    fn id(&self) -> &[ModuleId] {
        &self.id
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
            warn!("Invalid workspace ID");
            return;
        };

        target_workspace.is_focused = new_workspace.is_focused;
    }
}

impl Workspace {
    fn view(
        &self,
        style: &CommonStyle,
        config: &WorkspacesConfig,
    ) -> iced::Element<'_, crate::bar::BarEvent> {
        let background = if self.is_focused {
            Some(Background::Color(config.focused_color))
        } else {
            style.background.map(Background::Color)
        };

        let text_color = style.foreground.unwrap_or(Color::WHITE);
        let border = style.border;

        button(self.format(&config.format))
            .style(move |_old, _arg2| button::Style {
                background,
                text_color,
                border,
                ..Default::default()
            })
            .into()
    }

    fn format(&self, format: &str) -> Text<'_> {
        const NAME: &str = "{name}";
        const INDEX: &str = "{index}";

        let name = self.name.as_deref().unwrap_or_default();
        let content = format
            .replace(NAME, name)
            .replace(INDEX, &self.idx.to_string());

        text(content)
    }
}

fn try_create_backend() -> Option<Box<dyn IpcBackend>> {
    if let Some(niri) = <NiriIpcBackend as IpcBackend>::try_create() {
        info!("[Workspaces] using Niri backend");
        return Some(Box::new(niri));
    }

    None
}

fn worker(module_id: ModuleId) -> impl Stream<Item = ModuleUpdate> {
    stream::channel(0, async move |mut output| {
        let Some(mut ipc_backend) = try_create_backend() else {
            warn!("Unsupported compositor");
            return;
        };
        loop {
            let Some(event) = ipc_backend.next_event() else {
                continue;
            };
            send_data(&mut output, module_id, event).await;
        }
    })
}
