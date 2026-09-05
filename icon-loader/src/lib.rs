mod base_dirs;
mod theme;

use std::error::Error;
use std::fmt::{Debug, Display};
use std::path::PathBuf;

use crate::base_dirs::get_base_directories;
use crate::theme::{Theme, ThemeName, get_current_theme_name};

#[derive(Debug, Clone, Default)]
pub struct IconLoader {
    themes: Vec<Theme>,
}

#[derive(Clone)]
pub enum IconLoaderError {
    NoIconDirectory,
    OrphanTheme { name: String, parent: String },
}

#[derive(Clone, Copy, Debug)]
pub enum IconSize {
    Any,
    Exact(u16),
}

impl Debug for IconLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

impl Display for IconLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OrphanTheme { name, parent } => write!(
                f,
                "Theme {name} inherits from {parent} but {parent} could not be located"
            ),
            Self::NoIconDirectory => write!(f, "Could not locate any icon directory"),
        }
    }
}

impl Error for IconLoaderError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        match self {
            IconLoaderError::NoIconDirectory => "Could not locate any icon directory",
            IconLoaderError::OrphanTheme {
                name: _name,
                parent: _parent,
            } => "Attempted to inherit from a non-existant theme",
        }
    }
}

impl IconLoader {
    /// Creates a new `IconLoader`, its icon location and current theme are inferred based on the
    /// freedesktop icon theme specification
    ///
    /// # Errors
    /// This factory function will return an error if it can't infer the current theme or the
    /// inferred theme is considered to be invalid based on the freedesktop icon theme specification
    pub fn new() -> Result<Self, IconLoaderError> {
        let base_directories = get_base_directories();
        let current_theme = get_current_theme_name();

        let mut themes = vec![
            Theme::load(&current_theme, &base_directories)
                .ok_or(IconLoaderError::NoIconDirectory)?,
        ];

        let mut index = 0;
        while index < themes.len() {
            let parents = themes[index].get_parents();
            let parent_themes: Vec<Theme> = parents
                .iter()
                .map(|name| {
                    Theme::load(name, &base_directories).ok_or(IconLoaderError::OrphanTheme {
                        name: themes[index].get_name().0.clone(),
                        parent: name.0.clone(),
                    })
                })
                .collect::<Result<Vec<Theme>, IconLoaderError>>()?;

            themes.extend(parent_themes);
            index += 1;
        }

        Ok(Self { themes })
    }

    #[must_use]
    pub fn query_uncached(&self, icon_name: &str, icon_size: IconSize) -> Option<PathBuf> {
        let mut search_queue = vec![&self.themes[0]];
        loop {
            if search_queue.is_empty() {
                break;
            }

            let theme = search_queue[0];
            if let Some(maybe_icon) = theme.get_icon(icon_name, icon_size) {
                return Some(maybe_icon);
            }

            let fallbacks = theme
                .get_parents()
                .iter()
                .map(|theme| self.get_theme_with_name(theme));
            search_queue.extend(fallbacks);

            search_queue.remove(0);
        }

        None
    }

    fn get_theme_with_name(&self, theme_name: &ThemeName) -> &Theme {
        if let Some(matched) = self
            .themes
            .iter()
            .find(|theme| theme.get_name() == theme_name)
        {
            return matched;
        }

        self.get_theme_with_name(&ThemeName::default())
    }
}
