pub mod battery;

use std::fmt::Debug;
use std::{any::TypeId, sync::Arc};

use downcast_rs::{DowncastSync, impl_downcast};
use iced::{Element, Subscription};

use crate::bar::BarEvent;

pub struct ModuleUpdate(pub TypeId, pub Arc<dyn ModuleData>);

impl Debug for ModuleUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Update designated for module {:?}", self.0)
    }
}

pub trait ModuleData: DowncastSync {}
impl_downcast!(sync ModuleData);

pub trait Module: DowncastSync {
    fn update(&mut self, update_data: Arc<dyn ModuleData>);
    fn view(&self) -> Element<'_, BarEvent>;
    fn subscription(&self) -> Option<Subscription<ModuleUpdate>>;
}
