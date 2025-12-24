use crate::tui::{Flavor, Theme};
use crate::types::StatsSnapshot;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct PowerWidget<'a> {
    snapshot: &'a StatsSnapshot,
    theme: &'a Theme,
    flavor: &'a Flavor,
}

impl<'a> PowerWidget<'a> {
    pub fn new(snapshot: &'a StatsSnapshot, theme: &'a Theme, flavor: &'a Flavor) -> Self {
        Self {
            snapshot,
            theme,
            flavor,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.flavor.power_level_title()))
            .title_style(self.theme.header)
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        // Different display for arrival rate mode vs closed model
        let is_arrival_rate_mode = self.snapshot.vus_max > 0;

        let mut lines = if is_arrival_rate_mode {
            // Open model (arrival rate) - show achieved vs target
            let achieved_rate = self.snapshot.rolling_rps;
            let target_rate = self.snapshot.target_rate as f64;
            let rate_diff = if target_rate > 0.0 {
                ((achieved_rate - target_rate) / target_rate * 100.0).abs()
            } else {
                0.0
            };

            let rank = self.flavor.power_rank(achieved_rate);
            let rank_style = if achieved_rate > 9000.0 {
                self.theme.highlight
            } else {
                self.theme.muted
            };

            // Calculate dropped percentage
            let total_iterations = self.snapshot.total_requests + self.snapshot.dropped_iterations;
            let dropped_pct = if total_iterations > 0 {
                self.snapshot.dropped_iterations as f64 / total_iterations as f64 * 100.0
            } else {
                0.0
            };

            vec![
                Line::from(vec![
                    Span::styled("Load Model:  ", self.theme.normal),
                    Span::styled("Open (arrival rate)", self.theme.muted),
                ]),
                Line::from(vec![
                    Span::styled("Achieved (1s):", self.theme.normal),
                    Span::styled(format!("{:>6.0}", achieved_rate), 
                        if rate_diff > 10.0 { self.theme.warning } else { self.theme.success }),
                    Span::raw("  "),
                    Span::styled(format!("[{}]", rank), rank_style),
                ]),
                Line::from(vec![
                    Span::styled("Target RPS:  ", self.theme.normal),
                    Span::styled(format!("{:>6}", self.snapshot.target_rate), self.theme.highlight),
                    Span::styled("  Dropped: ", self.theme.normal),
                    Span::styled(
                        format!("{} ({:.2}%)", self.snapshot.dropped_iterations, dropped_pct),
                        if self.snapshot.dropped_iterations > 0 { self.theme.error } else { self.theme.success },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("VUs:         ", self.theme.normal),
                    Span::styled(
                        format!("{:>4}/{:<4}", self.snapshot.vus_active, self.snapshot.vus_max),
                        if self.snapshot.vus_active >= self.snapshot.vus_max { self.theme.warning } else { self.theme.normal },
                    ),
                    Span::styled("  Errors: ", self.theme.normal),
                    Span::styled(
                        format!("{:.2}%", self.snapshot.error_rate * 100.0),
                        if self.snapshot.error_rate > 0.05 { self.theme.error }
                        else if self.snapshot.error_rate > 0.01 { self.theme.warning }
                        else { self.theme.success },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Total:       ", self.theme.normal),
                    Span::styled(format!("{:>7}", format_number(self.snapshot.total_requests)), self.theme.normal),
                ]),
            ]
        } else {
            // Closed model (VU-driven)
            let rank = self.flavor.power_rank(self.snapshot.rolling_rps);
            let rank_style = if self.snapshot.rolling_rps > 9000.0 {
                self.theme.highlight
            } else {
                self.theme.muted
            };

            vec![
                Line::from(vec![
                    Span::styled("Load Model:  ", self.theme.normal),
                    Span::styled("Closed (VU-driven)", self.theme.muted),
                ]),
                Line::from(vec![
                    Span::styled("Rolling RPS: ", self.theme.normal),
                    Span::styled(format!("{:>8.0}", self.snapshot.rolling_rps), self.theme.highlight),
                    Span::raw("  "),
                    Span::styled(format!("[{}]", rank), rank_style),
                ]),
                Line::from(vec![
                    Span::styled("Total:       ", self.theme.normal),
                    Span::styled(format!("{:>8}", format_number(self.snapshot.total_requests)), self.theme.normal),
                ]),
                Line::from(vec![
                    Span::styled("Errors:      ", self.theme.normal),
                    Span::styled(
                        format!("{:>8} ({:.2}%)", format_number(self.snapshot.failed), self.snapshot.error_rate * 100.0),
                        if self.snapshot.error_rate > 0.05 { self.theme.error }
                        else if self.snapshot.error_rate > 0.01 { self.theme.warning }
                        else { self.theme.success },
                    ),
                ]),
            ]
        };

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            render_sparkline(&self.snapshot.timeline),
            self.theme.muted,
        )));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn render_sparkline(timeline: &[crate::types::TimelineBucket]) -> String {
    if timeline.is_empty() {
        return String::new();
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max_requests = timeline.iter().map(|b| b.requests).max().unwrap_or(1).max(1);

    timeline
        .iter()
        .take(20)
        .map(|bucket| {
            let idx = ((bucket.requests as f64 / max_requests as f64) * 7.0) as usize;
            chars[idx.min(7)]
        })
        .collect()
}
