//! Component Factory - Unified factory for creating TUI components
//!
//! Centralizes component creation to eliminate the 66+ repeated component calls
//! across TUI screens, providing consistent UI and reducing code duplication.

use ratatui::{
    layout::{Alignment, Rect},
    text::Line,
    widgets::{Block, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::{ComponentConfig, ComponentType, StyleManager};

/// Factory for creating standardized TUI components
pub struct ComponentFactory;

impl ComponentFactory {
    /// Create a title paragraph
    pub fn title(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .style(StyleManager::title())
            .alignment(Alignment::Center)
            .block(Self::default_block())
    }

    /// Create a title paragraph with custom styling
    pub fn title_with_config(
        text: impl Into<String>,
        config: ComponentConfig,
    ) -> Paragraph<'static> {
        let mut paragraph = Paragraph::new(text.into())
            .style(StyleManager::title())
            .alignment(config.alignment.unwrap_or(Alignment::Center));

        if config.borders {
            paragraph = paragraph.block(Self::block_from_config(&config));
        }

        if config.wrap {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }

        paragraph
    }

    /// Create a status paragraph
    pub fn status(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .block(Self::default_block().title("Status"))
            .wrap(Wrap { trim: true })
    }

    /// Create an error paragraph
    pub fn error(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .style(StyleManager::error())
            .block(Self::default_block().title("Error"))
            .wrap(Wrap { trim: true })
    }

    /// Create a success paragraph
    pub fn success(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .style(StyleManager::success())
            .block(Self::default_block().title("Success"))
            .wrap(Wrap { trim: true })
    }

    /// Create a warning paragraph
    pub fn warning(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .style(StyleManager::warning())
            .block(Self::default_block().title("Warning"))
            .wrap(Wrap { trim: true })
    }

    /// Create a help paragraph
    pub fn help(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .style(StyleManager::muted())
            .alignment(Alignment::Center)
            .block(Self::default_block().title("Help"))
            .wrap(Wrap { trim: true })
    }

    /// Create a details paragraph
    pub fn details(lines: Vec<Line<'static>>) -> Paragraph<'static> {
        Paragraph::new(lines)
            .block(Self::default_block().title("Details"))
            .wrap(Wrap { trim: true })
    }

    /// Create a details paragraph from text
    pub fn details_text(text: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(text.into())
            .block(Self::default_block().title("Details"))
            .wrap(Wrap { trim: true })
    }

    /// Create a progress gauge
    pub fn progress(percent: u16, label: impl Into<String>) -> Gauge<'static> {
        Gauge::default()
            .percent(percent)
            .label(label.into())
            .style(StyleManager::success())
            .block(Self::default_block())
    }

    /// Create a list component
    pub fn list(items: Vec<ListItem<'static>>) -> List<'static> {
        List::new(items)
            .block(Self::default_block())
            .highlight_style(StyleManager::selected())
            .highlight_symbol("▶ ")
    }

    /// Create a list with custom title
    pub fn list_with_title(
        items: Vec<ListItem<'static>>,
        title: impl Into<String>,
    ) -> List<'static> {
        List::new(items)
            .block(Self::default_block().title(title.into()))
            .highlight_style(StyleManager::selected())
            .highlight_symbol("▶ ")
    }

    /// Create a table component
    pub fn table(
        rows: Vec<Row<'static>>,
        widths: Vec<ratatui::layout::Constraint>,
    ) -> Table<'static> {
        Table::new(rows, widths)
            .block(Self::default_block())
            .column_spacing(1)
    }

    /// Create a table with header and title
    pub fn table_with_header(
        rows: Vec<Row<'static>>,
        header: Row<'static>,
        widths: Vec<ratatui::layout::Constraint>,
        title: Option<String>,
    ) -> Table<'static> {
        let mut table = Table::new(rows, widths)
            .header(header.style(StyleManager::title()))
            .column_spacing(1)
            .block(Self::default_block());

        if let Some(title_str) = title {
            table = table.block(Self::default_block().title(title_str));
        }

        table
    }

    /// Create empty state paragraph
    pub fn empty_state(message: impl Into<String>) -> Paragraph<'static> {
        Paragraph::new(message.into())
            .style(StyleManager::muted())
            .alignment(Alignment::Center)
            .block(Self::default_block())
            .wrap(Wrap { trim: true })
    }

    /// Create loading indicator
    pub fn loading(message: impl Into<String>) -> Paragraph<'static> {
        let text = format!("⏳ {}", message.into());
        Paragraph::new(text)
            .style(StyleManager::info())
            .alignment(Alignment::Center)
            .block(Self::default_block())
    }

    /// Create confirmation dialog
    pub fn confirm_dialog(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Paragraph<'static> {
        let content = format!("{}\n\n{}", title.into(), message.into());
        Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(Self::default_block().title("Confirm"))
            .wrap(Wrap { trim: true })
    }

    /// Create info dialog
    pub fn info_dialog(title: impl Into<String>, message: impl Into<String>) -> Paragraph<'static> {
        let content = format!("{}\n\n{}", title.into(), message.into());
        Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(Self::default_block().title("Information"))
            .wrap(Wrap { trim: true })
    }

    /// Render dialog overlay centered in frame
    pub fn render_dialog_overlay(
        frame: &mut Frame,
        area: Rect,
        content: Paragraph<'static>,
        width: u16,
        height: u16,
    ) {
        let dialog_width = width.min(area.width.saturating_sub(4));
        let dialog_height = height.min(area.height.saturating_sub(4));

        let dialog_area = Rect {
            x: area.x + (area.width - dialog_width) / 2,
            y: area.y + (area.height - dialog_height) / 2,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear background
        frame.render_widget(Clear, dialog_area);
        frame.render_widget(content, dialog_area);
    }

    /// Create component from configuration
    pub fn from_config(
        component_type: ComponentType,
        config: ComponentConfig,
    ) -> Box<dyn ComponentRenderer> {
        match component_type {
            ComponentType::Title => Box::new(TitleComponent::new(config)),
            ComponentType::Status => Box::new(StatusComponent::new(config)),
            ComponentType::Error => Box::new(ErrorComponent::new(config)),
            ComponentType::Help => Box::new(HelpComponent::new(config)),
            ComponentType::Details => Box::new(DetailsComponent::new(config)),
            ComponentType::Progress => Box::new(ProgressComponent::new(config)),
            _ => Box::new(TextComponent::new(config)),
        }
    }

    /// Helper to create default block
    fn default_block() -> Block<'static> {
        Block::default().borders(StyleManager::block_borders())
    }

    /// Helper to create block from config
    fn block_from_config(config: &ComponentConfig) -> Block<'static> {
        let mut block = Block::default();

        if config.borders {
            block = block.borders(StyleManager::block_borders());
        } else {
            block = block.borders(StyleManager::block_no_borders());
        }

        if let Some(ref title) = config.title {
            block = block.title(title.clone());
        }

        block
    }
}

/// Trait for renderable components
pub trait ComponentRenderer {
    fn render(&self, frame: &mut Frame, area: Rect);
}

/// Common text component
pub struct TextComponent {
    paragraph: Paragraph<'static>,
}

impl TextComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let text = config.content.as_ref().cloned().unwrap_or_default();
        let style = match config.style.as_deref() {
            Some("error") => StyleManager::error(),
            Some("warning") => StyleManager::warning(),
            Some("success") => StyleManager::success(),
            Some("muted") => StyleManager::muted(),
            _ => StyleManager::default(),
        };

        let mut paragraph = Paragraph::new(text).style(style);

        if config.wrap {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }

        paragraph = paragraph.block(ComponentFactory::block_from_config(&config));

        Self { paragraph }
    }
}

impl ComponentRenderer for TextComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.paragraph.clone(), area);
    }
}

/// Title component
pub struct TitleComponent {
    paragraph: Paragraph<'static>,
}

impl TitleComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let text = config.content.as_ref().cloned().unwrap_or_default();
        let paragraph = Paragraph::new(text)
            .style(StyleManager::title())
            .alignment(config.alignment.unwrap_or(Alignment::Center))
            .block(ComponentFactory::block_from_config(&config));

        Self { paragraph }
    }
}

impl ComponentRenderer for TitleComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.paragraph.clone(), area);
    }
}

/// Status component
pub struct StatusComponent {
    paragraph: Paragraph<'static>,
}

impl StatusComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let text = config.content.unwrap_or_else(|| "Ready".to_string());
        let title = config.title.unwrap_or_else(|| "Status".to_string());
        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(StyleManager::block_borders())
                .title(title),
        );

        Self { paragraph }
    }
}

impl ComponentRenderer for StatusComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.paragraph.clone(), area);
    }
}

/// Error component
pub struct ErrorComponent {
    paragraph: Paragraph<'static>,
}

impl ErrorComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let text = config.content.as_ref().cloned().unwrap_or_default();
        let title = config.title.unwrap_or_else(|| "Error".to_string());
        let paragraph = Paragraph::new(text)
            .style(StyleManager::error())
            .block(
                Block::default()
                    .borders(StyleManager::block_borders())
                    .title(title),
            )
            .wrap(Wrap { trim: true });

        Self { paragraph }
    }
}

impl ComponentRenderer for ErrorComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.paragraph.clone(), area);
    }
}

/// Help component
pub struct HelpComponent {
    paragraph: Paragraph<'static>,
}

impl HelpComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let text = config.content.as_ref().cloned().unwrap_or_default();
        let title = config.title.unwrap_or_else(|| "Help".to_string());
        let paragraph = Paragraph::new(text)
            .style(StyleManager::muted())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(StyleManager::block_borders())
                    .title(title),
            )
            .wrap(Wrap { trim: true });

        Self { paragraph }
    }
}

impl ComponentRenderer for HelpComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.paragraph.clone(), area);
    }
}

/// Details component
pub struct DetailsComponent {
    paragraph: Paragraph<'static>,
}

impl DetailsComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let text = config.content.as_ref().cloned().unwrap_or_default();
        let title = config.title.unwrap_or_else(|| "Details".to_string());
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(StyleManager::block_borders())
                    .title(title),
            )
            .wrap(Wrap { trim: true });

        Self { paragraph }
    }
}

impl ComponentRenderer for DetailsComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.paragraph.clone(), area);
    }
}

/// Progress component
pub struct ProgressComponent {
    gauge: Gauge<'static>,
}

impl ProgressComponent {
    pub fn new(config: ComponentConfig) -> Self {
        let percent = config
            .content
            .as_ref()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);

        let title = config.title.unwrap_or_else(|| "Progress".to_string());
        let gauge = Gauge::default().percent(percent).block(
            Block::default()
                .borders(StyleManager::block_borders())
                .title(title),
        );

        Self { gauge }
    }
}

impl ComponentRenderer for ProgressComponent {
    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.gauge.clone(), area);
    }
}
