use std::{collections::HashMap, rc::Rc};

use iced::Theme;
use toml::{Table, Value, value::Array};

use crate::{
    bar::Bar,
    logger::{error, warn},
    modules::{Module, battery::Battery, cpu::Cpu},
};

type ModuleFactoryFunction = fn(&Table) -> Rc<dyn Module>;
type ModuleInternalName = &'static str;

const FACTORY_FUNCTIONS: [(ModuleInternalName, ModuleFactoryFunction); 2] = [
    // Module name   Module implementation
    ("battery", <Battery as Module>::new_or_default),
    ("cpu", <Cpu as Module>::new_or_default),
];

impl From<&Table> for Bar {
    fn from(value: &Table) -> Self {
        let Some(bar_config) = value.get("bar") else {
            warn("Bar config empty, nothing to do");
            return Bar::default();
        };

        let module_registry = parse_modules(&value);
        let get_module = |key| -> Option<Vec<Rc<dyn Module>>> {
            let modules = bar_config.get(key)?;
            let modules = modules.as_array()?;
            Some(get_module_list(modules, &module_registry))
        };

        let left_modules = get_module("left_modules").unwrap_or_default();
        let center_modules = get_module("center_modules").unwrap_or_default();
        let right_modules = get_module("right_modules").unwrap_or_default();

        let theme = get_theme(&value);

        Self {
            left_modules,
            center_modules,
            right_modules,
            theme,
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
            warn("Found non-string value in module array, skipping");
            continue;
        };
        if let Some(module) = module_registry.get(module_name) {
            modules.push(module.clone());
        } else {
            warn(format!("Couldn't locate module with name {module_name}"));
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
        let (_module_name, init_function) = FACTORY_FUNCTIONS
            .iter()
            .find(|func| func.0 == key)
            .expect("Implement custom modules");

        let Some(table) = value.as_table() else {
            error(format!("item {key} must be a table, skipping"));
            continue;
        };

        modules.insert(key.clone(), init_function(table));
    }

    modules
}

fn get_theme(table: &Table) -> Theme {
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
            error(format!("Unrecognized theme {other}"));
            return None;
        }
    })
}

#[cfg(test)]
mod test {
    const TEST_CONFIG_FILE: &str = include_str!("../examples/configs/test-config.toml");
    use toml::Table;

    use crate::bar::Bar;

    #[test]
    fn test_config_parsing() {
        let table = TEST_CONFIG_FILE.parse::<Table>().unwrap();
        let bar = Bar::from(&table);
    }
}
