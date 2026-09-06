use std::collections::HashMap;
use std::sync::LazyLock;

use iced::Theme;
use log::{error, warn};
use toml::{Table, Value, value::Array};

use crate::bar::{Bar, BarConfig};
use crate::modules::{NamedModule, reexports::*};

type ModuleFactoryFunction = fn(&Table) -> Box<dyn Module>;
type ModuleInternalName = &'static str;

fn register_module<T: Module + NamedModule>() -> (ModuleInternalName, ModuleFactoryFunction) {
    let name = T::name();
    let factory_function = T::new_or_default;

    (name, factory_function)
}

pub const DEFAULT_BAR_SIZE: u32 = 32;
pub const DEFAULT_SPACING: u32 = 0;

static FACTORY_FUNCTIONS: LazyLock<HashMap<ModuleInternalName, ModuleFactoryFunction>> =
    LazyLock::new(|| {
        let modules = vec![
            register_module::<Battery>(),
            register_module::<Cpu>(),
            register_module::<Workspaces>(),
            register_module::<Clock>(),
            register_module::<Memory>(),
            register_module::<Temperature>(),
            // TODO: Implement bluetooth module
            register_module::<PipeWire>(),
            // TODO: Implement backlight module
            // TODO: Implement idle inhibitor module
        ];

        HashMap::from_iter(modules)
    });

impl From<&Table> for Bar {
    fn from(value: &Table) -> Self {
        let Some(bar_config) = value.get("bar") else {
            warn!("Bar config empty, nothing to do");
            return Bar::default();
        };

        let get_module_list = |key| -> Option<Array> {
            let modules = bar_config.get(key)?;
            Some(modules.as_array()?.clone())
        };

        let left_modules_name = get_module_list("left_modules").unwrap_or_default();
        let left_modules = get_modules(value, &left_modules_name);

        let center_modules_name = get_module_list("center_modules").unwrap_or_default();
        let center_modules = get_modules(value, &center_modules_name);

        let right_modules_name = get_module_list("right_modules").unwrap_or_default();
        let right_modules = get_modules(value, &right_modules_name);

        let theme = get_theme(bar_config);

        let get_property = |property_name: &str| -> Option<u32> {
            let value = bar_config.get(property_name)?;
            let int = value.as_integer()?;
            u32::try_from(int).ok()
        };

        let bar_size = get_property("size").unwrap_or(DEFAULT_BAR_SIZE);
        let spacing = get_property("spacing").unwrap_or(DEFAULT_SPACING);

        let config = BarConfig::new(theme, bar_size, spacing);

        Self {
            left_modules,
            center_modules,
            right_modules,
            config,
            outputs: Vec::new(),
        }
    }
}

pub(crate) fn get_modules(config_table: &Table, module_name_list: &Array) -> Vec<Box<dyn Module>> {
    let mut modules = Vec::new();
    for module in module_name_list {
        let Some(module_name) = module.as_str() else {
            warn!("{module:?} isn't a string, which it should");
            continue;
        };
        let Some(module_init_function) = FACTORY_FUNCTIONS.get(module_name) else {
            if let Some(custom_module) = get_custom_module(config_table, module_name) {
                modules.push(custom_module);
            }
            continue;
        };
        let module_config = || -> Option<Table> {
            let module_config = config_table.get(module_name)?;
            module_config.as_table().cloned()
        }()
        .unwrap_or_default();

        modules.push(module_init_function(&module_config));
    }

    modules
}

fn has_valid_config(module_name: &str, is_group: bool, is_script: bool) -> bool {
    let check = [is_group, is_script]
        .iter()
        .map(|bool| u8::from(*bool))
        .sum::<u8>();
    if check == 0 {
        error!("{module_name} should be either a group or a script module, it is neither, skipping");
        return false;
    }
    if check > 1 {
        error!("{module_name} should be either a group or a script module, it is both, skipping");
        return false;
    }

    true
}

fn get_custom_module(config_table: &Table, module_name: &str) -> Option<Box<dyn Module>> {
    let Some(module_config) = config_table.get(module_name) else {
        error!(
            "{module_name} is used but no definition for it was found, define it by adding a table named {module_name} to your configuration."
        );
        return None;
    };
    let Some(module_config) = module_config.as_table() else {
        error!("{module_name} has a non table value for configuration");
        return None;
    };

    let is_group = module_config
        .get("group")
        .is_some_and(|module_list| module_list.as_array().is_some());
    let is_script = module_config
        .get("command")
        .is_some_and(|command| command.as_str().is_some());

    if !has_valid_config(module_name, is_group, is_script) {
        return None;
    }

    if is_script {
        return Some(Script::new_or_default(module_config));
    }

    if is_group {
        return Some(Box::new(Group::new(module_config, config_table)));
    }

    None
}

fn get_theme(table: &Value) -> Theme {
    let theme = match table.get("theme") {
        Some(Value::String(theme)) => get_builtin_theme(theme),
        _ => None,
    };

    theme.unwrap_or(Theme::GruvboxDark)
}

fn get_builtin_theme(theme_name: &str) -> Option<Theme> {
    Some(match theme_name {
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        "dracula" => Theme::Dracula,
        "nord" => Theme::Nord,
        "solarized light" => Theme::SolarizedLight,
        "solarized dark" => Theme::SolarizedDark,
        "gruvbox light" => Theme::GruvboxLight,
        "gruvbox dark" => Theme::GruvboxDark,
        "catppuccin latte" => Theme::CatppuccinLatte,
        "catppuccin frappe" => Theme::CatppuccinFrappe,
        "catppuccin macchiato" => Theme::CatppuccinMacchiato,
        "catppuccin mocha" => Theme::CatppuccinMocha,
        "tokyo night" => Theme::TokyoNight,
        "tokyo night storm" => Theme::TokyoNightStorm,
        "tokyo night light" => Theme::TokyoNightLight,
        "kanagawa wave" => Theme::KanagawaWave,
        "kanagawa dragon" => Theme::KanagawaDragon,
        "kanagawa lotus" => Theme::KanagawaLotus,
        "moonfly" => Theme::Moonfly,
        "nightfly" => Theme::Nightfly,
        "oxocarbon" => Theme::Oxocarbon,
        "ferra" => Theme::Ferra,
        other => {
            error!("Unrecognized theme {other}");
            return None;
        }
    })
}
