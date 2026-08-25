use std::rc::Rc;

use iced::Alignment::Center;
use iced::Length::Fill;
use iced::event::wayland::{self, OutputEvent};
use iced::platform_specific::shell::commands::layer_surface::get_layer_surface;
use iced::runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use iced::widget::{container, row};
use iced::window::Id;
use iced::{Element, Event, Subscription, Task, Theme};

use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

use crate::modules::{Module, ModuleUpdate};
use crate::{BAR_PARAMETER, CONFIG};

pub struct Bar {
    pub(crate) left_modules: Vec<Rc<dyn Module>>,
    pub(crate) center_modules: Vec<Rc<dyn Module>>,
    pub(crate) right_modules: Vec<Rc<dyn Module>>,
    pub(crate) theme: Theme,
    pub(crate) outputs: Vec<Output>,
}

#[derive(Clone)]
pub struct Output {
    display: WlOutput,
    pub info: Option<OutputInfo>,
    id: Id,
}

#[derive(Debug, Clone)]
pub enum BarEvent {
    ModuleUpdate(ModuleUpdate),
    OutputUpdate(OutputEvent, WlOutput),
    OutputReady(WlOutput, Id),
}

impl Bar {
    pub fn start() -> Self {
        Self::from(&*CONFIG)
    }

    pub fn update(&mut self, message: BarEvent) -> Task<BarEvent> {
        match message {
            BarEvent::ModuleUpdate(module_update) => {
                let type_id = module_update.0;
                self.all_modules_mut()
                    .filter(|module| module.type_id() == type_id)
                    .for_each(|module| module.update(module_update.1.clone()));
                Task::none()
            }
            BarEvent::OutputUpdate(event, output) => self.handle_output_event(event, output),
            BarEvent::OutputReady(output, id) => self.create_client(output, id),
        }
    }

    pub fn view(&self, _window_id: Id) -> Element<'_, BarEvent> {
        let current_output = self
            .outputs
            .iter()
            .find(|output| output.id == _window_id)
            .expect("Trying to use a WlOutput before initializing it");

        let left_modules = row(self
            .left_modules
            .iter()
            .map(|module| module.view(&current_output)))
        .height(Fill)
        .align_y(Center);
        let center_modules = row(self
            .center_modules
            .iter()
            .map(|module| module.view(&current_output)))
        .height(Fill)
        .align_y(Center);
        let right_modules = row(self
            .right_modules
            .iter()
            .map(|module| module.view(&current_output)))
        .height(Fill)
        .align_y(Center);

        row![
            container(left_modules).align_left(Fill),
            container(center_modules).center(Fill),
            container(right_modules).align_right(Fill)
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<BarEvent> {
        let subscriptions = self.all_modules().filter_map(Module::subscription);

        Subscription::batch([
            Subscription::batch(subscriptions).map(BarEvent::ModuleUpdate),
            compositor_events(),
        ])
    }

    pub fn theme(&self, _window_id: Id) -> Theme {
        self.theme.clone()
    }

    fn all_modules(&self) -> impl Iterator<Item = &dyn Module> {
        self.left_modules
            .iter()
            .chain(self.center_modules.iter())
            .chain(self.right_modules.iter())
            .map(|rc| &**rc)
    }

    fn all_modules_mut(&mut self) -> impl Iterator<Item = &mut dyn Module> {
        self.left_modules
            .iter_mut()
            .chain(self.center_modules.iter_mut())
            .chain(self.right_modules.iter_mut())
            .map(|rc| Rc::<dyn Module + 'static>::get_mut(rc).unwrap())
    }

    fn handle_output_event(
        &mut self,
        output_event: OutputEvent,
        wl_display: WlOutput,
    ) -> Task<BarEvent> {
        match output_event {
            OutputEvent::Created(maybe_info) => {
                let output_id = Id::unique();
                let output = Output {
                    id: output_id,
                    display: wl_display.clone(),
                    info: maybe_info,
                };
                self.outputs.push(output);
                return Task::done(BarEvent::OutputReady(wl_display, output_id));
            }
            OutputEvent::Removed => {
                let position = self
                    .outputs
                    .iter()
                    .position(|output| output.display == wl_display);
                if let Some(position) = position {
                    self.outputs.get_mut(position).unwrap().display.release();
                    self.outputs.swap_remove(position);
                }
            }
            OutputEvent::InfoUpdate(new_info) => {
                let position = self
                    .outputs
                    .iter()
                    .position(|output| output.display == wl_display);
                if let Some(position) = position {
                    self.outputs.get_mut(position).unwrap().info = Some(new_info);
                }
            }
        }
        Task::none()
    }

    fn create_client(&mut self, wl_display: WlOutput, id: Id) -> Task<BarEvent> {
        get_layer_surface(SctkLayerSurfaceSettings {
            id,
            size: Some((Some(BAR_PARAMETER.bar_size), Some(BAR_PARAMETER.bar_size))),
            anchor: Anchor::LEFT | Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: BAR_PARAMETER.bar_size.cast_signed(),
            layer: Layer::Top,
            output: IcedOutput::Output(wl_display),
            ..Default::default()
        })
    }
}

fn compositor_events() -> Subscription<BarEvent> {
    iced::event::listen_with(|event, _var_2, _window_id| {
        if let Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
            wayland::Event::Output(event, output),
        )) = event
        {
            return Some(BarEvent::OutputUpdate(event, output));
        }

        None
    })
}

impl Default for Bar {
    fn default() -> Self {
        Bar {
            left_modules: Vec::new(),
            center_modules: Vec::new(),
            right_modules: Vec::new(),
            theme: Theme::Dark,
            outputs: Vec::new(),
        }
    }
}
