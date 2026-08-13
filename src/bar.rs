use iced::Alignment::Center;
use iced::Length::Fill;
use iced::{Element, Task};
use iced::widget::{row, text};
use iced_layershell::to_layer_message;

#[derive(Default)]
pub struct Bar {}

#[to_layer_message]
#[derive(Debug)]
pub enum BarEvent {}

impl Bar {
    pub fn namespace() -> String {
        String::from("Minibar")
    }

    pub fn update(&mut self, message: BarEvent) -> Task<BarEvent> {
        todo!()
    }

    pub fn view(&self) -> Element<'_, BarEvent> {
        row![
            text("Hello, World!")
        ].height(Fill).align_y(Center).into()
    }
}
