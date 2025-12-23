use crate::tui::widgets::{LatencyWidget, PowerWidget, StatusWidget};
use crate::tui::{Flavor, Theme};
use crate::types::{RunState, StatsSnapshot};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Duration;

pub fn render(
    frame: &mut Frame,
    snapshot: &StatsSnapshot,
    state: RunState,
    config_url: &str,
    config_concurrency: u32,
    config_duration: Duration,
    theme: &Theme,
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
        config_url,
        config_concurrency,
        config_duration,
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

    render_footer(frame, chunks[3], state, theme, flavor);
}

fn render_header(
    frame: &mut Frame,
    area: Rect,
    snapshot: &StatsSnapshot,
    state: RunState,
    url: &str,
    concurrency: u32,
    duration: Duration,
    theme: &Theme,
    flavor: &Flavor,
) {
    let elapsed = snapshot.elapsed.as_secs();
    let total = duration.as_secs();

    let title = if state == RunState::Running {
        flavor.status_running(concurrency)
    } else {
        flavor.title().to_string()
    };

    let truncated_url = if url.len() > 40 {
        format!("{}...", &url[..37])
    } else {
        url.to_string()
    };

    let header_line = Line::from(vec![
        Span::styled(format!("  {}    ", title), theme.title),
        Span::styled(truncated_url, theme.normal),
        Span::styled(
            format!("    [{:02}:{:02}/{:02}:{:02}]", elapsed / 60, elapsed % 60, total / 60, total % 60),
            theme.muted,
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);

    let paragraph = Paragraph::new(header_line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: RunState, theme: &Theme, flavor: &Flavor) {
    let status = match state {
        RunState::Initializing => Span::styled(flavor.status_initializing(), theme.muted),
        RunState::Running => Span::styled("Running...", theme.success),
        RunState::Paused => Span::styled("Paused", theme.warning),
        RunState::Stopping => Span::styled("Stopping...", theme.warning),
        RunState::Completed => Span::styled(flavor.status_completed(), theme.success),
        RunState::Cancelled => Span::styled(flavor.status_cancelled(), theme.warning),
        RunState::Error => Span::styled("Error!", theme.error),
    };

    let help = Span::styled("  [q]uit  [s]ave  [?]help", theme.muted);

    let line = Line::from(vec![help, Span::raw("    "), status]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
