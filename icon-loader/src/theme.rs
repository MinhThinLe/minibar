use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use crate::IconSize;

const ICON_THEME_KEY: &str = "gtk-icon-theme-name";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ThemeName(pub String);

#[derive(Debug, Clone)]
struct IconDir {
    icon_size: u16,
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Theme {
    /// The theme's name
    name: ThemeName,
    /// The name of themes which this theme inherits from
    parents: Vec<ThemeName>,
    /// Where to look for an icon in case it isn't present in cache
    directories: Vec<IconDir>,
}

impl Default for ThemeName {
    fn default() -> Self {
        Self(String::from("hicolor"))
    }
}

impl Theme {
    pub(crate) fn load(name: &ThemeName, base_dirs: &[PathBuf]) -> Option<Self> {
        let path = base_dirs.iter().find_map(|dir| {
            let index_theme_file = dir.join(&name.0).join("index.theme");
            index_theme_file
                .try_exists()
                .ok()?
                .then_some(index_theme_file)
        })?;

        let index_theme_content = fs::read_to_string(&path).ok()?;
        let parents = get_parents(&index_theme_content);
        let directories = get_icon_directories(&index_theme_content, path.parent().unwrap());

        Some(Self {
            name: name.clone(),
            parents,
            directories,
        })
    }

    pub(crate) fn get_icon(&self, icon_name: &str, icon_size: IconSize) -> Option<PathBuf> {
        for directory in &self.directories {
            match icon_size {
                IconSize::Any => (),
                IconSize::Exact(icon_size) => {
                    if directory.icon_size < icon_size {
                        continue;
                    }
                    if directory.icon_size > icon_size {
                        return None;
                    }
                }
            }
            let maybe_icon = directory.get_icon(icon_name);
            if maybe_icon.is_some() {
                return maybe_icon;
            }
        }

        None
    }

    pub(crate) fn get_parents(&self) -> &[ThemeName] {
        &self.parents
    }

    pub(crate) fn get_name(&self) -> &ThemeName {
        &self.name
    }
}

impl IconDir {
    fn get_icon(&self, icon_name: &str) -> Option<PathBuf> {
        const EXTENSIONS: [&str; 3] = ["png", "svg", "xpm"];
        for extension in EXTENSIONS {
            let file_name = format!("{icon_name}.{extension}");
            let file_path = self.path.join(file_name);
            if let Ok(true) = file_path.try_exists() {
                return Some(file_path);
            }
        }

        None
    }
}

fn get_icon_directories(theme_index: &str, base_dir: &Path) -> Vec<IconDir> {
    let mut sub_dirs = Vec::new();

    let mut sub_directory = PathBuf::default();
    let mut icon_size = u16::default();

    for line in theme_index.lines() {
        if marks_table(line) {
            let table_name = table_name(line);
            if table_name == "Icon Theme" {
                continue;
            }
            if sub_directory == PathBuf::default() {
                sub_directory = PathBuf::from(table_name);
                continue;
            }

            let icon_dir = IconDir {
                path: base_dir.join(&sub_directory),
                icon_size,
            };

            sub_dirs.push(icon_dir);

            sub_directory = PathBuf::from(table_name);

            continue;
        }
        if line.starts_with("Size") {
            icon_size = line["Size=".len()..].trim().parse().unwrap_or(0);
        }
    }

    sub_dirs
}

fn marks_table(line: &str) -> bool {
    line.starts_with('[') && line.ends_with(']')
}

fn table_name(line: &str) -> &str {
    &line[1..line.len() - 1]
}

fn get_parents(theme_index: &str) -> Vec<ThemeName> {
    const PARENT_KEY: &str = "Inherits";
    scan_buffer(theme_index, PARENT_KEY)
        .map(|string| {
            string
                .split(',')
                .map(|parent| ThemeName(parent.to_string()))
                .collect()
        })
        .unwrap_or(vec![])
}

pub fn get_current_theme_name() -> ThemeName {
    if let Some(theme) = from_gtk4() {
        return ThemeName(theme);
    }
    if let Some(theme) = from_gtk3() {
        return ThemeName(theme);
    }
    if let Some(theme) = from_gtk2() {
        return ThemeName(theme);
    }

    ThemeName::default()
}

/// Get the icon-theme property from /org/gnome/desktop/interface/ using dconf
fn from_gtk4() -> Option<String> {
    const DCONF_VERB: &str = "read";
    const PROPERTY: &str = "/org/gnome/desktop/interface/icon-theme";

    let output = Command::new("dconf")
        .arg(DCONF_VERB)
        .arg(PROPERTY)
        .output()
        .ok()?;

    let output_length = output.stdout.len();
    if output_length <= 3 {
        return None;
    }

    // dconf's command line output comes in the following form:
    // '{Theme name}' where {Theme name} is the name of the current theme
    // Armed with that knowledge, the reason for the two constants' existence is left as an excercise
    // for the reader
    String::from_utf8(output.stdout[1..output_length - 2].to_vec()).ok()
}

fn from_gtk3() -> Option<String> {
    const CONFIG_DIR: &str = "gtk-3.0";
    const CONFIG_FILE: &str = "settings.ini";

    if let Some(config_dir) = gtk_3_user_config().map(|dir| dir.join(CONFIG_DIR).join(CONFIG_FILE))
        && let Ok(true) = config_dir.try_exists()
        && let Ok(buffer) = fs::read_to_string(config_dir)
        && let Some(theme) = scan_buffer(&buffer, ICON_THEME_KEY)
    {
        return Some(theme);
    }

    let system_theme = system_config().join(CONFIG_DIR).join(CONFIG_FILE);
    if let Ok(true) = system_theme.try_exists()
        && let Ok(buffer) = fs::read_to_string(system_theme)
        && let Some(theme) = scan_buffer(&buffer, ICON_THEME_KEY)
    {
        return Some(theme);
    }

    None
}

fn from_gtk2() -> Option<String> {
    const CONFIG_DIR: &str = "gtk-2.0";
    const CONFIG_FILE: &str = "gtkrc";

    if let Some(user_config) = gtk_2_user_config()
        && let Ok(true) = user_config.try_exists()
        && let Ok(buffer) = fs::read_to_string(user_config)
        && let Some(theme) = scan_buffer(&buffer, ICON_THEME_KEY)
    {
        return Some(theme);
    }

    let system_config = system_config().join(CONFIG_DIR).join(CONFIG_FILE);
    if let Ok(true) = system_config.try_exists()
        && let Ok(buffer) = fs::read_to_string(system_config)
        && let Some(theme) = scan_buffer(&buffer, ICON_THEME_KEY)
    {
        return Some(theme);
    }

    None
}

pub(crate) fn scan_buffer(buffer: &str, key: &str) -> Option<String> {
    let value = buffer.lines().find_map(|line| {
        line.starts_with(key)
            .then_some(line.split_once('=')?.1.trim().to_string())
    })?;
    // Theme names must only consist of ASCII characters as per the [XDG Icon Theme
    // Specification](https://specifications.freedesktop.org/icon-theme/latest/). It it then safe to
    // assume that each character only takes up a single byte
    if value.chars().next()? == '"' {
        return Some(value[1..(value.len() - 1)].to_string());
    }
    Some(value)
}

fn gtk_2_user_config() -> Option<PathBuf> {
    const GTKRC: &str = "gtkrc-2.0";
    Some(env::home_dir()?.join(GTKRC))
}

fn gtk_3_user_config() -> Option<PathBuf> {
    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config_home));
    }

    let home_dir = env::home_dir()?;
    Some(home_dir.join(".config"))
}

fn system_config() -> PathBuf {
    PathBuf::from("/etc")
}
