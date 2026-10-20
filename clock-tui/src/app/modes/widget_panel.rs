//! The command-widget area shared by clock and timer modes: the configured
//! widgets, their show/hide state, the keys that drive them, and the split of
//! the screen between the big digits and the widgets.

use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
};

use crate::config::ClockWidgetConfig;

use super::clock_widget::{clock_height_for_size, clock_size_for_area, ClockTheme, ClockWidgets};

pub(crate) struct WidgetPanel {
    widgets: ClockWidgets,
    visible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PanelLayout {
    pub(crate) digits_area: Rect,
    pub(crate) widgets_area: Rect,
    pub(crate) digits_size: u16,
}

impl WidgetPanel {
    pub(crate) fn new(configs: Vec<ClockWidgetConfig>, themes: Vec<String>) -> Self {
        Self {
            widgets: ClockWidgets::new(configs, themes),
            visible: true,
        }
    }

    pub(crate) fn tick(&mut self) {
        self.widgets.tick();
    }

    /// Handles the keys that belong to the widget area (`T` theme, `g` group,
    /// `z` clock-only layout, Home/End/PageUp/PageDown scroll). Returns false
    /// for any other key so the mode can use it.
    pub(crate) fn on_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('T') => self.widgets.cycle_theme(),
            KeyCode::Char('g') => self.widgets.cycle_group(),
            KeyCode::Char('z') => self.toggle(),
            KeyCode::Home => self.widgets.scroll_active_to_top(),
            KeyCode::End => self.widgets.scroll_active_to_bottom(),
            KeyCode::PageUp => self.widgets.scroll_active_by_page(-1),
            KeyCode::PageDown => self.widgets.scroll_active_by_page(1),
            _ => return false,
        }
        true
    }

    pub(crate) fn on_popup_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.widgets.close_popup(),
            KeyCode::Up => self.widgets.scroll_popup(-1),
            KeyCode::Down => self.widgets.scroll_popup(1),
            KeyCode::PageUp => self.widgets.scroll_popup(-10),
            KeyCode::PageDown => self.widgets.scroll_popup(10),
            KeyCode::Home => self.widgets.scroll_active_to_top(),
            KeyCode::End => self.widgets.scroll_active_to_bottom(),
            _ => {}
        }
    }

    pub(crate) fn open_popup_action(&mut self, key: char) -> bool {
        self.widgets.open_popup_action(key)
    }

    pub(crate) fn has_popup_open(&self) -> bool {
        self.widgets.has_popup_open()
    }

    pub(crate) fn scroll_at(&mut self, column: u16, row: u16, delta: i16) {
        self.widgets.scroll_at(column, row, delta);
    }

    pub(crate) fn theme(&self, base_style: Style) -> ClockTheme {
        self.widgets.current_clock_theme(base_style)
    }

    /// Where the digits and the widgets go, or `None` when there is nothing to
    /// show and the digits keep the whole screen. `has_footer` reserves the
    /// status line a mode prints below its digits (the timer's preset hint).
    pub(crate) fn layout(
        &self,
        area: Rect,
        text_len: usize,
        has_header: bool,
        has_footer: bool,
    ) -> Option<PanelLayout> {
        (self.visible && !self.widgets.is_empty())
            .then(|| panel_layout(area, text_len, has_header, has_footer))
    }

    /// Draws the widgets and, over everything, an open popup. Call it after the
    /// digits so the popup stays on top.
    pub(crate) fn render(
        &mut self,
        layout: PanelLayout,
        area: Rect,
        buf: &mut Buffer,
        theme: ClockTheme,
    ) {
        self.widgets.render(layout.widgets_area, area, buf, theme);
        self.widgets.render_popup(area, buf, theme);
    }

    fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.widgets.hide_all();
        }
    }

    #[cfg(test)]
    pub(crate) fn visible_for_test(&self) -> bool {
        self.visible
    }

    #[cfg(test)]
    pub(crate) fn current_theme_for_test(&self) -> &str {
        self.widgets.current_theme_for_test()
    }
}

fn panel_layout(area: Rect, text_len: usize, has_header: bool, has_footer: bool) -> PanelLayout {
    let digits_height_budget = digits_height_budget(area.height);
    // `render_centered` leaves a blank line between the digits and the footer.
    let footer_lines = if has_footer { 2 } else { 0 };
    let sizing_area = Rect {
        height: digits_height_budget.saturating_sub(footer_lines),
        ..area
    };
    let digits_size = clock_size_for_area(text_len, sizing_area, has_header);
    let digits_height = clock_height_for_size(digits_size, has_header)
        .saturating_add(footer_lines)
        .min(digits_height_budget)
        .min(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(digits_height), Constraint::Min(0)])
        .split(area);

    PanelLayout {
        digits_area: chunks[0],
        widgets_area: chunks[1],
        digits_size,
    }
}

fn digits_height_budget(area_height: u16) -> u16 {
    if area_height == 0 {
        0
    } else {
        (area_height / 2).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widget() -> ClockWidgetConfig {
        ClockWidgetConfig {
            title: Some("Status".to_string()),
            command: vec!["true".to_string()],
            popup_actions: Vec::new(),
            refresh_secs: 900,
            timeout_secs: 30,
            position: Default::default(),
            group: None,
        }
    }

    #[test]
    fn widgets_get_extra_height_when_width_limits_square_clock() {
        let layout = panel_layout(Rect::new(0, 0, 80, 80), 8, true, false);

        assert_eq!(layout.digits_size, 1);
        assert_eq!(layout.digits_area.height, 9);
        assert_eq!(layout.widgets_area.height, 71);
    }

    #[test]
    fn wide_clock_height_is_capped_to_top_half() {
        let layout = panel_layout(Rect::new(0, 0, 500, 50), 8, true, false);

        assert_eq!(layout.digits_size, 4);
        assert!(layout.digits_area.height <= 25);
        assert_eq!(layout.digits_area.height, 24);
        assert_eq!(layout.widgets_area.height, 26);
    }

    #[test]
    fn clock_layout_leaves_vertical_breathing_without_header() {
        let layout = panel_layout(Rect::new(0, 0, 500, 50), 8, false, false);

        assert_eq!(layout.digits_size, 4);
        assert_eq!(layout.digits_area.height, 22);
        assert_eq!(layout.widgets_area.height, 28);
    }

    #[test]
    fn footer_reserves_two_lines_below_the_digits() {
        let without = panel_layout(Rect::new(0, 0, 500, 50), 8, true, false);
        let with = panel_layout(Rect::new(0, 0, 500, 50), 8, true, true);

        assert_eq!(
            with.digits_area.height,
            clock_height_for_size(with.digits_size, true) + 2
        );
        assert!(with.digits_area.height <= 25);
        assert!(with.digits_size <= without.digits_size);
    }

    #[test]
    fn clock_layout_handles_tiny_areas() {
        let layout = panel_layout(Rect::new(0, 0, 10, 1), 8, true, false);

        assert_eq!(layout.digits_area.height, 1);
        assert_eq!(layout.widgets_area.height, 0);
    }

    #[test]
    fn no_layout_without_widgets_or_when_hidden() {
        let area = Rect::new(0, 0, 120, 40);
        assert_eq!(
            WidgetPanel::new(Vec::new(), Vec::new()).layout(area, 8, true, false),
            None
        );

        let mut panel = WidgetPanel::new(vec![widget()], Vec::new());
        assert!(panel.layout(area, 8, true, false).is_some());

        assert!(panel.on_key(KeyCode::Char('z')));
        assert_eq!(panel.layout(area, 8, true, false), None);
    }

    #[test]
    fn on_key_claims_only_widget_keys() {
        let mut panel = WidgetPanel::new(Vec::new(), vec!["default".into(), "nerv".into()]);

        assert!(panel.on_key(KeyCode::Char('T')));
        assert_eq!(panel.current_theme_for_test(), "nerv");
        assert!(!panel.on_key(KeyCode::Char('m')));
        assert!(!panel.on_key(KeyCode::Enter));
        assert!(!panel.on_key(KeyCode::Char(' ')));
    }
}
