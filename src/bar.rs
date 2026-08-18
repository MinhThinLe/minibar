use iced::Alignment::Center;
use iced::Length::Fill;
use iced::widget::{container, row};
use iced::{Element, Subscription, Task};
use iced_layershell::to_layer_message;

use crate::modules::battery::Battery;
use crate::modules::{Module, ModuleUpdate};

pub struct Bar {
    left_modules: Vec<Box<dyn Module>>,
    center_modules: Vec<Box<dyn Module>>,
    right_modules: Vec<Box<dyn Module>>,
}

#[to_layer_message]
#[derive(Debug)]
pub enum BarEvent {
    ModuleUpdate(ModuleUpdate),
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            center_modules: vec![],
            right_modules: vec![Box::new(Battery::default())],
            left_modules: vec![],
        }
    }
}

impl Bar {
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
            other => println!("Unhandled event: {other:?}"),
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
        let subscriptions = self
            .all_modules()
            .map(|module| module.subscription())
            .filter(|subscrition| subscrition.is_some())
            .map(|subscription| subscription.unwrap());

        Subscription::batch(subscriptions).map(BarEvent::ModuleUpdate)
    }

    fn all_modules(&self) -> impl Iterator<Item = &Box<dyn Module>> {
        self.left_modules
            .iter()
            .chain(self.center_modules.iter())
            .chain(self.right_modules.iter())
    }

    fn all_modules_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Module>> {
        self.left_modules
            .iter_mut()
            .chain(self.center_modules.iter_mut())
            .chain(self.right_modules.iter_mut())
    }
}
