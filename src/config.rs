use std::sync::LazyLock;
use std::{collections::HashMap, rc::Rc};

use iced::Theme;
use log::{error, warn};
use toml::{Table, Value, value::Array};

use crate::bar::Bar;
use crate::modules::{NamedModule, reexports::*};

type ModuleFactoryFunction = fn(&Table) -> Rc<dyn Module>;
type ModuleInternalName = &'static str;

fn register_module<T: Module + NamedModule>() -> (ModuleInternalName, ModuleFactoryFunction) {
    let name = T::name();
    let factory_function = T::new_or_default;

    (name, factory_function)
}

static FACTORY_FUNCTIONS: LazyLock<HashMap<ModuleInternalName, ModuleFactoryFunction>> =
    LazyLock::new(|| {
        let modules = vec![
            register_module::<Battery>(),
            register_module::<Cpu>(),
            register_module::<Workspaces>(),
            register_module::<Clock>(),
            register_module::<Memory>(),
            register_module::<Temperature>(),
            register_module::<SysTray>(),
            // TODO: Implement bluetooth module
            // TODO: Implement group module
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

        let module_registry = parse_modules(value);
        let get_module = |key| -> Option<Vec<Rc<dyn Module>>> {
            let modules = bar_config.get(key)?;
            let modules = modules.as_array()?;
            Some(get_module_list(modules, &module_registry))
        };

        let left_modules = get_module("left_modules").unwrap_or_default();
        let center_modules = get_module("center_modules").unwrap_or_default();
        let right_modules = get_module("right_modules").unwrap_or_default();

        let theme = get_theme(bar_config);

        Self {
            left_modules,
            center_modules,
            right_modules,
            theme,
            outputs: Vec::new(),
        }
    }
}

fn get_module_list(
    array: &Array,
    module_registry: &HashMap<String, Rc<dyn Module>>,
) -> Vec<Rc<dyn Module>> {
    let mut modules = Vec::with_capacity(array.len());
    for item in array {
        let Some(module_name) = item.as_str() else {
            warn!("Found non-string value in module array, skipping");
            continue;
        };
        if let Some(module) = module_registry.get(module_name) {
            modules.push(module.clone());
        } else {
            // todo!("Implement custom modules");
            // warn!("Couldn't locate module with name {module_name}");
        }
    }
    modules
}

fn parse_modules(table: &Table) -> HashMap<String, Rc<dyn Module>> {
    let mut modules: HashMap<String, Rc<dyn Module>> = FACTORY_FUNCTIONS
        .iter()
        .map(|(module_name, init_function)| (module_name.to_string(), init_function(&Table::new())))
        .collect();

    for (key, value) in table {
        if key == "bar" {
            continue;
        }

        let Some(init_function) = FACTORY_FUNCTIONS.get(key.as_str()) else {
            error!("Module {key} doesn't exists and custom modules aren't implemented yet");
            continue;
        };

        let Some(table) = value.as_table() else {
            error!("item {key} must be a table, skipping");
            continue;
        };

        modules.insert(key.clone(), init_function(table));
    }

    modules
}

fn get_theme(table: &Value) -> Theme {
    let theme = match table.get("theme") {
        Some(Value::String(theme)) => get_builtin_theme(theme),
        _ => None,
    };

    theme.unwrap_or(Theme::GruvboxDark)
}

fn get_builtin_theme(theme_name: &str) -> Option<Theme> {
    println!("{theme_name}");
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
