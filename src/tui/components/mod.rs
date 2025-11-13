//! TUI Components - Unified component factory for reducing code duplication
//!
//! This module provides a centralized component factory that eliminates the 66
//! repeated rendering calls across TUI screens, following DRY principles.

pub mod component_factory;
pub mod layout_builder;
pub mod style_manager;

pub use component_factory::ComponentFactory;
pub use layout_builder::LayoutBuilder;
pub use style_manager::StyleManager;

/// Common component types for standardized UI elements
#[derive(Debug, Clone)]
pub enum ComponentType {
    Title,
    Status,
    Help,
    Error,
    Details,
    Progress,
    List,
    Table,
    Form,
}

/// Standard layout constraints for consistent UI
#[derive(Debug, Clone)]
pub enum LayoutConstraint {
    Fixed(u16),
    Percentage(u16),
    Min(u16),
    Max(u16),
}

/// Component configuration for flexible creation
#[derive(Debug, Clone)]
pub struct ComponentConfig {
    pub title: Option<String>,
    pub content: Option<String>,
    pub style: Option<String>,
    pub borders: bool,
    pub wrap: bool,
    pub alignment: Option<ratatui::layout::Alignment>,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            title: None,
            content: None,
            style: None,
            borders: true,
            wrap: true,
            alignment: None,
        }
    }
}

impl ComponentConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn style(mut self, style: impl Into<String>) -> Self {
        self.style = Some(style.into());
        self
    }

    pub fn borders(mut self, borders: bool) -> Self {
        self.borders = borders;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn alignment(mut self, alignment: ratatui::layout::Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}
