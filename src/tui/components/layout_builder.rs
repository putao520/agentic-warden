//! Layout Builder - Fluent interface for creating consistent layouts
//!
//! Reduces the repeated Layout::default().direction().constraints().split() pattern
//! that appears in every TUI screen, eliminating code duplication.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// Fluent layout builder for consistent TUI layouts
pub struct LayoutBuilder {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: u16,
}

impl LayoutBuilder {
    /// Create new layout builder with vertical direction (default)
    pub fn new() -> Self {
        Self {
            direction: Direction::Vertical,
            constraints: Vec::new(),
            margin: 0,
        }
    }

    /// Create new layout builder with specified direction
    pub fn with_direction(direction: Direction) -> Self {
        Self {
            direction,
            constraints: Vec::new(),
            margin: 0,
        }
    }

    /// Set layout direction
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Add fixed height/width constraint
    pub fn fixed(mut self, size: u16) -> Self {
        self.constraints.push(Constraint::Length(size));
        self
    }

    /// Add percentage constraint
    pub fn percentage(mut self, percent: u16) -> Self {
        self.constraints.push(Constraint::Percentage(percent));
        self
    }

    /// Add minimum size constraint
    pub fn min(mut self, size: u16) -> Self {
        self.constraints.push(Constraint::Min(size));
        self
    }

    /// Add maximum size constraint
    pub fn max(mut self, size: u16) -> Self {
        self.constraints.push(Constraint::Max(size));
        self
    }

    /// Add proportional constraint (1-100)
    pub fn ratio(mut self, ratio: u16) -> Self {
        self.constraints.push(Constraint::Ratio(ratio as u32, 100));
        self
    }

    /// Add remaining space constraint
    pub fn remaining(mut self) -> Self {
        self.constraints.push(Constraint::Min(0));
        self
    }

    /// Set margin around layout
    pub fn margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }

    /// Build the layout and split the given area
    pub fn split(self, area: Rect) -> Vec<Rect> {
        if self.constraints.is_empty() {
            vec![area]
        } else if self.margin > 0 {
            let inner_area = Self::apply_margin(area, self.margin);
            Layout::default()
                .direction(self.direction)
                .constraints(self.constraints)
                .split(inner_area)
                .to_vec()
        } else {
            Layout::default()
                .direction(self.direction)
                .constraints(self.constraints)
                .split(area)
                .to_vec()
        }
    }

    /// Apply margin to an area
    fn apply_margin(area: Rect, margin: u16) -> Rect {
        let new_x = area.x + margin;
        let new_y = area.y + margin;
        let new_width = area.width.saturating_sub(2 * margin);
        let new_height = area.height.saturating_sub(2 * margin);

        Rect::new(new_x, new_y, new_width, new_height)
    }

    /// Build and render widgets directly to frame
    pub fn render_and_split<F>(self, frame: &mut Frame, area: Rect, render_fn: F) -> Vec<Rect>
    where
        F: FnOnce(&mut Frame, Vec<Rect>),
    {
        let chunks = self.split(area);
        render_fn(frame, chunks.clone());
        chunks
    }
}

impl Default for LayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Common layout presets for consistent UI patterns
impl LayoutBuilder {
    /// Create standard three-row layout (header, body, footer)
    pub fn three_row(header_height: u16, footer_height: u16) -> Self {
        Self::new()
            .fixed(header_height)
            .remaining() // Body
            .fixed(footer_height)
    }

    /// Create standard two-row layout (header, body)
    pub fn two_row(header_height: u16) -> Self {
        Self::new().fixed(header_height).remaining()
    }

    /// Create standard three-column layout (left, center, right)
    pub fn three_column(left_percent: u16, right_percent: u16) -> Self {
        let center_percent = 100u16.saturating_sub(left_percent + right_percent);
        Self::with_direction(Direction::Horizontal)
            .percentage(left_percent)
            .percentage(center_percent)
            .percentage(right_percent)
    }

    /// Create standard two-column layout (left, right)
    pub fn two_column(left_percent: u16) -> Self {
        Self::with_direction(Direction::Horizontal)
            .percentage(left_percent)
            .percentage(100u16.saturating_sub(left_percent))
    }

    /// Create dialog layout (title, content, buttons)
    pub fn dialog(title_height: u16, button_height: u16) -> Self {
        Self::new()
            .fixed(title_height)
            .remaining() // Content
            .fixed(button_height)
    }

    /// Create screen layout (header, body, status, help)
    pub fn screen(header_height: u16, status_height: u16, help_height: u16) -> Self {
        Self::new()
            .fixed(header_height)
            .remaining() // Body
            .fixed(status_height)
            .fixed(help_height)
    }

    /// Create tabbed layout (tabs, content)
    pub fn tabbed(tab_height: u16) -> Self {
        Self::new().fixed(tab_height).remaining()
    }
}

/// Extension trait for Rect to provide common layout operations
pub trait RectExt {
    /// Split into two equal parts horizontally
    fn split_horizontal_half(self) -> (Rect, Rect);

    /// Split into two equal parts vertically
    fn split_vertical_half(self) -> (Rect, Rect);

    /// Split horizontally with percentage
    fn split_horizontal_percentage(self, left_percent: u16) -> (Rect, Rect);

    /// Split vertically with percentage
    fn split_vertical_percentage(self, top_percent: u16) -> (Rect, Rect);
}

impl RectExt for Rect {
    fn split_horizontal_half(self) -> (Rect, Rect) {
        let rects = LayoutBuilder::two_column(50).split(self);
        (rects[0], rects[1])
    }

    fn split_vertical_half(self) -> (Rect, Rect) {
        let rects = LayoutBuilder::new()
            .fixed(self.height / 2)
            .remaining()
            .split(self);
        (rects[0], rects[1])
    }

    fn split_horizontal_percentage(self, left_percent: u16) -> (Rect, Rect) {
        let rects = LayoutBuilder::two_column(left_percent).split(self);
        (rects[0], rects[1])
    }

    fn split_vertical_percentage(self, top_percent: u16) -> (Rect, Rect) {
        let rects = LayoutBuilder::new()
            .percentage(top_percent)
            .remaining()
            .split(self);
        (rects[0], rects[1])
    }
}
