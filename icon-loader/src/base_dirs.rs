use std::env;
use std::path::PathBuf;

pub fn get_base_directories() -> Vec<PathBuf> {
    let mut icon_directories = vec![];

    if let Some(icon_dir) = from_home_dir() {
        icon_directories.push(icon_dir);
    }
    icon_directories.extend(from_xdg_data_dirs());

    icon_directories
}

fn from_home_dir() -> Option<PathBuf> {
    let icon_dir = env::home_dir()?.join(".icons");
    icon_dir
        .try_exists()
        .ok()
        .and_then(|exists| exists.then_some(icon_dir))
}

fn from_xdg_data_dirs() -> Vec<PathBuf> {
    const SEPARATOR: char = ':';
    const ICONS_DIR: &str = "icons";

    let mut icon_directories = vec![];

    let Ok(config_dirs) = env::var("XDG_DATA_DIRS") else {
        return icon_directories;
    };

    for config_dir in config_dirs.split(SEPARATOR).map(PathBuf::from) {
        let icon_dir = config_dir.join(ICONS_DIR);
        if let Ok(true) = icon_dir.try_exists() {
            icon_directories.push(icon_dir);
        }
    }

    icon_directories
}
