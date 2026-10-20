use std::{cell::RefCell, cmp::min, process::Command, time::Instant};

use crate::app::modes::pause::Pause;
use crate::clock_text::font::bricks::BricksFont;
use crate::clock_text::ClockText;
use chrono::Duration;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::{
    elapsed_since, format_duration, render_centered, should_flash, DurationFormat, PAUSED_FOOTER,
};

const DEFAULT_TIMER_MINUTES: i64 = 5;

/// A named set of durations the timer can switch to with `m`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerPreset {
    pub name: String,
    pub durations: Vec<Duration>,
    /// Empty keeps the titles the timer was built with.
    pub titles: Vec<String>,
}

pub struct Timer {
    pub size: u16,
    pub style: Style,
    pub repeat: bool,
    pub durations: Vec<Duration>,
    pub titles: Vec<String>,
    pub execute: Vec<String>,
    base_titles: Vec<String>,
    presets: Vec<TimerPreset>,
    preset_idx: usize,
    auto_quit: bool,
    format: DurationFormat,
    passed: Duration,
    started_at: Option<Instant>,
    execute_result: RefCell<Option<String>>,
    flash_state: RefCell<bool>,
}

impl Timer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size: u16,
        style: Style,
        durations: Vec<Duration>,
        titles: Vec<String>,
        repeat: bool,
        format: DurationFormat,
        paused: bool,
        auto_quit: bool,
        execute: Vec<String>,
    ) -> Self {
        Self {
            size,
            style,
            durations: normalize_durations(durations),
            base_titles: titles.clone(),
            titles,
            presets: Vec::new(),
            preset_idx: 0,
            repeat,
            execute,
            auto_quit,
            format,
            passed: Duration::zero(),
            started_at: (!paused).then(Instant::now),
            execute_result: RefCell::new(None),
            flash_state: RefCell::new(false),
        }
    }

    /// Loads the first preset and waits paused, so the user can pick one with
    /// `m` before starting. An empty list leaves the timer as built.
    pub(crate) fn with_presets(mut self, presets: Vec<TimerPreset>) -> Self {
        self.presets = presets;
        if !self.presets.is_empty() {
            self.apply_preset(0);
        }
        self
    }

    pub(crate) fn active_preset_name(&self) -> Option<&str> {
        self.presets
            .get(self.preset_idx)
            .map(|preset| preset.name.as_str())
    }

    /// Switches to the next preset and restarts it from zero. Only allowed
    /// while paused, so a running session is never reset by accident.
    pub(crate) fn cycle_preset(&mut self) -> bool {
        if self.presets.len() < 2 || !self.is_paused() {
            return false;
        }
        self.apply_preset((self.preset_idx + 1) % self.presets.len());
        true
    }

    /// Cancels the session: back to the start of the current preset (or the
    /// configured durations), paused.
    pub(crate) fn reset(&mut self) {
        self.passed = Duration::zero();
        self.started_at = None;
        *self.execute_result.borrow_mut() = None;
    }

    fn apply_preset(&mut self, idx: usize) {
        let preset = &self.presets[idx];
        self.durations = normalize_durations(preset.durations.clone());
        self.titles = if preset.titles.is_empty() {
            self.base_titles.clone()
        } else {
            preset.titles.clone()
        };
        self.preset_idx = idx;
        self.reset();
    }

    fn footer(&self) -> Option<String> {
        if !self.is_paused() {
            return self.execute_result.borrow().clone();
        }
        match self.active_preset_name() {
            Some(name) if self.passed.is_zero() => Some(format!(
                "{name} (press <m> for the next preset, <ENTER> to start, <r> to reset)"
            )),
            _ => Some(PAUSED_FOOTER.to_string()),
        }
    }

    pub(crate) fn default_durations() -> Vec<Duration> {
        vec![Duration::minutes(DEFAULT_TIMER_MINUTES)]
    }

    pub(crate) fn remaining_time(&self) -> (Duration, usize) {
        if self.durations.is_empty() {
            return (Duration::zero(), 0);
        }

        let total_passed = if let Some(started_at) = self.started_at {
            self.passed + elapsed_since(started_at)
        } else {
            self.passed
        };

        let mut idx = 0;
        let mut next_checkpoint = self.durations[0];
        while next_checkpoint < total_passed {
            if idx >= self.durations.len() - 1 && !self.repeat {
                break;
            }
            idx = (idx + 1) % self.durations.len();
            next_checkpoint += self.durations[idx];
        }

        (next_checkpoint - total_passed, idx)
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.auto_quit && self.execute_result.borrow().is_some()
    }
}

fn normalize_durations(durations: Vec<Duration>) -> Vec<Duration> {
    if durations.is_empty() {
        Timer::default_durations()
    } else {
        durations
    }
}

fn execute(execute: &[String]) -> String {
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    let cmd_str = execute.join(" ");
    cmd.arg(cmd_str);
    let output = cmd.output();
    match output {
        Ok(output) => {
            if !output.status.success() {
                format!("[ERROR] {}", String::from_utf8_lossy(&output.stderr))
            } else {
                format!("[SUCCEED] {}", String::from_utf8_lossy(&output.stdout))
            }
        }
        Err(e) => {
            format!("[FAILED] {}", e)
        }
    }
}

impl Widget for &Timer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (remaining_time, idx) = self.remaining_time();

        if remaining_time < Duration::zero() {
            if self.execute_result.borrow().is_none() {
                if !self.execute.is_empty() {
                    let result = execute(&self.execute);
                    *self.execute_result.borrow_mut() = Some(result);
                } else {
                    *self.execute_result.borrow_mut() = Some("".to_owned())
                }
            }

            // Flash the screen when timer is done
            let should_flash = should_flash(remaining_time);
            *self.flash_state.borrow_mut() = should_flash;

            // Fill the entire area with the flash color
            let flash_style = if *self.flash_state.borrow() {
                Style::default().bg(Color::Green)
            } else {
                Style::default().bg(Color::Black)
            };

            // Fill the entire area with the flash color
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    buf[(x, y)].set_style(flash_style);
                }
            }

            // Only render the text during the visible phase
            if should_flash {
                let elapsed_time = -remaining_time; // Make the time positive
                let time_str = format_duration(elapsed_time, self.format);
                let header = if self.titles.is_empty() {
                    None
                } else {
                    Some(self.titles[min(idx, self.titles.len() - 1)].clone())
                };

                let font = BricksFont::new(self.size);
                let text = ClockText::new(
                    time_str.as_str().to_string(),
                    &font,
                    self.style.fg(Color::Black), // Make text visible on green background
                );

                let footer = self.footer();

                render_centered(area, buf, &text, header, footer, Style::default());
            }
        } else {
            // Normal rendering logic when timer has not reached zero
            let time_str = format_duration(remaining_time, self.format);
            let header = if self.titles.is_empty() {
                None
            } else {
                Some(self.titles[min(idx, self.titles.len() - 1)].clone())
            };

            let font = BricksFont::new(self.size);
            let text = ClockText::new(time_str.as_str().to_string(), &font, self.style);

            let footer = self.footer();

            render_centered(area, buf, &text, header, footer, Style::default());
        }
    }
}

impl Pause for Timer {
    fn is_paused(&self) -> bool {
        self.started_at.is_none()
    }

    fn pause(&mut self) {
        if let Some(started_at) = self.started_at {
            self.passed += elapsed_since(started_at);
            self.started_at = None;
        }
    }

    fn resume(&mut self) {
        if self.started_at.is_none() {
            self.started_at = Some(Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timer(paused: bool, titles: Vec<String>) -> Timer {
        Timer::new(
            1,
            Style::default(),
            vec![Duration::minutes(5)],
            titles,
            true,
            DurationFormat::HourMinSec,
            paused,
            false,
            Vec::new(),
        )
    }

    fn presets() -> Vec<TimerPreset> {
        vec![
            TimerPreset {
                name: "Pomodoro".to_string(),
                durations: vec![Duration::minutes(25), Duration::minutes(5)],
                titles: vec!["Focus".to_string(), "Break".to_string()],
            },
            TimerPreset {
                name: "Deep".to_string(),
                durations: vec![Duration::minutes(50), Duration::minutes(10)],
                titles: Vec::new(),
            },
        ]
    }

    #[test]
    fn presets_load_the_first_preset_and_wait_paused() {
        let timer = timer(false, Vec::new()).with_presets(presets());

        assert_eq!(timer.active_preset_name(), Some("Pomodoro"));
        assert_eq!(
            timer.durations,
            vec![Duration::minutes(25), Duration::minutes(5)]
        );
        assert_eq!(timer.titles, vec!["Focus", "Break"]);
        assert!(timer.is_paused());
        assert_eq!(timer.remaining_time(), (Duration::minutes(25), 0));
    }

    #[test]
    fn no_presets_leave_the_timer_untouched() {
        let timer = timer(false, vec!["Work".to_string()]).with_presets(Vec::new());

        assert_eq!(timer.active_preset_name(), None);
        assert_eq!(timer.durations, vec![Duration::minutes(5)]);
        assert_eq!(timer.titles, vec!["Work"]);
        assert!(!timer.is_paused());
    }

    #[test]
    fn cycle_preset_moves_to_the_next_preset_and_wraps_around() {
        let mut timer = timer(true, Vec::new()).with_presets(presets());

        assert!(timer.cycle_preset());
        assert_eq!(timer.active_preset_name(), Some("Deep"));
        assert_eq!(
            timer.durations,
            vec![Duration::minutes(50), Duration::minutes(10)]
        );

        assert!(timer.cycle_preset());
        assert_eq!(timer.active_preset_name(), Some("Pomodoro"));
    }

    #[test]
    fn a_preset_without_titles_falls_back_to_the_configured_titles() {
        let mut timer = timer(true, vec!["Work".to_string()]).with_presets(presets());

        timer.cycle_preset();

        assert_eq!(timer.titles, vec!["Work"]);
    }

    #[test]
    fn cycle_preset_is_ignored_while_the_timer_runs() {
        let mut timer = timer(true, Vec::new()).with_presets(presets());
        timer.resume();

        assert!(!timer.cycle_preset());
        assert_eq!(timer.active_preset_name(), Some("Pomodoro"));
        assert!(!timer.is_paused());
    }

    #[test]
    fn cycle_preset_after_a_pause_restarts_from_zero() {
        let mut timer = timer(true, Vec::new()).with_presets(presets());
        timer.resume();
        timer.pause();

        assert!(timer.cycle_preset());
        assert!(timer.is_paused());
        assert_eq!(timer.remaining_time(), (Duration::minutes(50), 0));
    }

    #[test]
    fn reset_returns_to_the_start_of_the_current_preset_paused() {
        let mut timer = timer(true, Vec::new()).with_presets(presets());
        timer.cycle_preset();
        timer.resume();

        timer.reset();

        assert!(timer.is_paused());
        assert_eq!(timer.active_preset_name(), Some("Deep"));
        assert_eq!(timer.remaining_time(), (Duration::minutes(50), 0));
    }

    #[test]
    fn reset_without_presets_restarts_the_durations_paused() {
        let mut timer = timer(false, Vec::new());

        timer.reset();

        assert!(timer.is_paused());
        assert_eq!(timer.remaining_time(), (Duration::minutes(5), 0));
    }

    #[test]
    fn footer_names_the_preset_and_its_keys_before_the_first_start() {
        let mut timer = timer(true, Vec::new()).with_presets(presets());

        let footer = timer.footer().unwrap();
        assert!(footer.contains("Pomodoro"), "{footer}");
        assert!(footer.contains("<m>"), "{footer}");
        assert!(footer.contains("<ENTER>"), "{footer}");

        timer.resume();
        assert_eq!(timer.footer(), None);

        timer.pause();
        assert_eq!(timer.footer().as_deref(), Some(PAUSED_FOOTER));
    }

    #[test]
    fn empty_duration_list_falls_back_to_default_timer() {
        let timer = Timer::new(
            1,
            Style::default(),
            Vec::new(),
            Vec::new(),
            false,
            DurationFormat::HourMinSec,
            true,
            false,
            Vec::new(),
        );

        assert_eq!(timer.durations, Timer::default_durations());
        assert_eq!(
            timer.remaining_time(),
            (Duration::minutes(DEFAULT_TIMER_MINUTES), 0)
        );
    }
}
