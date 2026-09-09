use crate::config::get_modules;

use super::*;

const DEFAULT_SPACING: u16 = 2;

pub struct Group {
    children: Vec<Box<dyn Module>>,
    ids: Vec<ModuleId>,
    style: CommonStyle,
    spacing: u16,
}

impl Module for Group {
    fn id(&self) -> &[ModuleId] {
        &self.ids
    }

    fn update(&mut self, module_update: ModuleUpdate) {
        let module_id = module_update.0;
        self.children
            .iter_mut()
            .filter(|child| child.id().contains(&module_id))
            .for_each(|child| child.update(module_update.clone()));
    }

    fn view(&self, output: &Output) -> Element<'_, BarEvent> {
        container(row(self.children.iter().map(|child| child.view(output))).spacing(self.spacing))
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
        let subscriptions = self
            .children
            .iter()
            .filter_map(|module| module.as_ref().subscription());

        Some(Subscription::batch(subscriptions))
    }

    fn new_or_default(_table: &Table) -> Box<dyn Module>
    where
        Self: Sized,
    {
        error!(
            "The new_or_default method on struct Group should not be called, use the dedicated Group::new() method in stead"
        );
        panic!()
    }
}

impl Group {
    pub fn new(module_config: &Table, global_config: &Table) -> Self {
        let modules = module_config
            .get("group")
            .expect("Group module should have a config field named `group`");
        let children = modules
            .as_array()
            .expect("Group module should have an array named `group` in its configuration");

        let children = get_modules(global_config, children);

        let ids = children
            .iter()
            .flat_map(|child| child.id().iter().copied())
            .collect();

        let style = CommonStyle::from(module_config);
        let spacing =
            get_int(module_config, "spacing").map_or(DEFAULT_SPACING, |spacing| spacing as u16);

        Self {
            children,
            ids,
            style,
            spacing,
        }
    }
}
