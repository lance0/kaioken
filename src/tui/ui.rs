use crate::tui::theme::ThemeMode;
use crate::tui::widgets::{LatencyWidget, PowerWidget, StatusWidget};
use crate::tui::{Flavor, Theme};
use crate::types::{RunPhase, RunState, StatsSnapshot};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    snapshot: &StatsSnapshot,
    state: RunState,
    phase: RunPhase,
    config_url: &str,
    config_concurrency: u32,
    config_duration: Duration,
    config_warmup: Duration,
    theme: &Theme,
    theme_mode: ThemeMode,
    flavor: &Flavor,
) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(
        frame,
        chunks[0],
        snapshot,
        state,
        phase,
        config_url,
        config_concurrency,
        config_duration,
        config_warmup,
        theme,
        flavor,
    );

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    PowerWidget::new(snapshot, theme, flavor).render(frame, middle[0]);
    LatencyWidget::new(snapshot, theme).render(frame, middle[1]);

    StatusWidget::new(snapshot, theme).render(frame, chunks[2]);

    render_footer(frame, chunks[3], state, phase, theme, theme_mode, flavor);
}

#[allow(clippy::too_many_arguments)]
fn render_header(
    frame: &mut Frame,
    area: Rect,
    snapshot: &StatsSnapshot,
    state: RunState,
    phase: RunPhase,
    url: &str,
    concurrency: u32,
    duration: Duration,
    warmup: Duration,
    theme: &Theme,
    flavor: &Flavor,
) {
    let elapsed = snapshot.elapsed.as_secs();
    let total = duration.as_secs();

    let title = if state == RunState::Running {
        if phase == RunPhase::Warmup {
            if flavor.serious {
                "Warming up...".to_string()
            } else {
                "Charging...".to_string()
            }
        } else {
            flavor.status_running(concurrency)
        }
    } else {
        flavor.title().to_string()
    };

    let truncated_url = if url.len() > 40 {
        format!("{}...", &url[..37])
    } else {
        url.to_string()
    };

    let time_display = if phase == RunPhase::Warmup && !warmup.is_zero() {
        let warmup_secs = warmup.as_secs();
        format!("    [warmup {:02}:{:02}]", warmup_secs / 60, warmup_secs % 60)
    } else {
        format!("    [{:02}:{:02}/{:02}:{:02}]", elapsed / 60, elapsed % 60, total / 60, total % 60)
    };

    let header_line = Line::from(vec![
        Span::styled(format!("  {}    ", title), theme.title),
        Span::styled(truncated_url, theme.normal),
        Span::styled(time_display, theme.muted),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);

    let paragraph = Paragraph::new(header_line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_footer(
    frame: &mut Frame,
    area: Rect,
    state: RunState,
    phase: RunPhase,
    theme: &Theme,
    theme_mode: ThemeMode,
    flavor: &Flavor,
) {
    let status = match state {
        RunState::Initializing => Span::styled(flavor.status_initializing(), theme.muted),
        RunState::Running => {
            if phase == RunPhase::Warmup {
                Span::styled("Warmup (not measuring)", theme.warning)
            } else {
                Span::styled("Running...", theme.success)
            }
        }
        RunState::Paused => Span::styled("Paused", theme.warning),
        RunState::Stopping => Span::styled("Stopping...", theme.warning),
        RunState::Completed => Span::styled(flavor.status_completed(), theme.success),
        RunState::Cancelled => Span::styled(flavor.status_cancelled(), theme.warning),
        RunState::Error => Span::styled("Error!", theme.error),
    };

    let theme_indicator = Span::styled(format!("[{}]", theme_mode.name()), theme.highlight);
    let help = Span::styled("  [q]uit  [s]ave  [t]heme", theme.muted);

    let line = Line::from(vec![theme_indicator, help, Span::raw("    "), status]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
