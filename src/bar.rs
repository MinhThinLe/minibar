use std::rc::Rc;

use iced::Alignment::Center;
use iced::Length::Fill;
use iced::widget::{container, row};
use iced::{Element, Subscription, Task, Theme};
use iced_layershell::to_layer_message;

use crate::CONFIG;
use crate::logger::warn;
use crate::modules::{Module, ModuleUpdate};

pub struct Bar {
    pub(crate) left_modules: Vec<Rc<dyn Module>>,
    pub(crate) center_modules: Vec<Rc<dyn Module>>,
    pub(crate) right_modules: Vec<Rc<dyn Module>>,
    pub(crate) theme: Theme,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum BarEvent {
    ModuleUpdate(ModuleUpdate),
}

impl Bar {
    pub fn start() -> Self {
        Self::from(&*CONFIG)
    }

    pub fn namespace() -> String {
        String::from("Minibar")
    }

    pub fn update(&mut self, message: BarEvent) -> Task<BarEvent> {
        match message {
            BarEvent::ModuleUpdate(module_update) => {
                let type_id = module_update.0;
                self.all_modules_mut()
                    .filter(|module| module.type_id() == type_id)
                    .for_each(|module| module.update(module_update.1.clone()));
            }
            other => warn(format!("Unhandled event: {other:?}")),
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, BarEvent> {
        let left_modules = row(self.left_modules.iter().map(|module| module.view()))
            .height(Fill)
            .align_y(Center);
        let center_modules = row(self.center_modules.iter().map(|module| module.view()))
            .height(Fill)
            .align_y(Center);
        let right_modules = row(self.right_modules.iter().map(|module| module.view()))
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

        Subscription::batch(subscriptions).map(BarEvent::ModuleUpdate)
    }

    pub fn theme(&self) -> Theme {
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
}

impl Default for Bar {
    fn default() -> Self {
        Bar {
            left_modules: Vec::new(),
            center_modules: Vec::new(),
            right_modules: Vec::new(),
            theme: Theme::Dark,
        }
    }
}
