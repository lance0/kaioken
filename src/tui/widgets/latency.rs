use crate::tui::Theme;
use crate::types::StatsSnapshot;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct LatencyWidget<'a> {
    snapshot: &'a StatsSnapshot,
    theme: &'a Theme,
}

impl<'a> LatencyWidget<'a> {
    pub fn new(snapshot: &'a StatsSnapshot, theme: &'a Theme) -> Self {
        Self { snapshot, theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Use corrected latency if available, otherwise fall back to wall-clock
        let use_corrected = self.snapshot.latency_correction_enabled
            && self.snapshot.corrected_latency_p50_us.is_some();

        let title = if use_corrected {
            " LATENCY (ms) [corrected] "
        } else {
            " LATENCY (ms) "
        };

        let block = Block::default()
            .title(title)
            .title_style(self.theme.header)
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let (p50, p90, p95, p99, p999) = if use_corrected {
            (
                self.snapshot.corrected_latency_p50_us.unwrap_or(0),
                self.snapshot.corrected_latency_p90_us.unwrap_or(0),
                self.snapshot.corrected_latency_p95_us.unwrap_or(0),
                self.snapshot.corrected_latency_p99_us.unwrap_or(0),
                self.snapshot.corrected_latency_p999_us.unwrap_or(0),
            )
        } else {
            (
                self.snapshot.latency_p50_us,
                self.snapshot.latency_p90_us,
                self.snapshot.latency_p95_us,
                self.snapshot.latency_p99_us,
                self.snapshot.latency_p999_us,
            )
        };

        let max_latency = p999.max(1) as f64;

        let percentiles = [
            ("p50", p50),
            ("p90", p90),
            ("p95", p95),
            ("p99", p99),
            ("p999", p999),
        ];

        let lines: Vec<Line> = percentiles
            .iter()
            .map(|(label, value)| {
                let ms = *value as f64 / 1000.0;
                let bar_width = ((*value as f64 / max_latency) * 15.0) as usize;
                let bar: String = "█".repeat(bar_width.min(15));

                let style = self.latency_style(ms);

                Line::from(vec![
                    Span::styled(format!("{:>4}: ", label), self.theme.normal),
                    Span::styled(format!("{:>6.0}", ms), style),
                    Span::raw("  "),
                    Span::styled(bar, self.theme.bar_filled),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn latency_style(&self, ms: f64) -> ratatui::style::Style {
        if ms > 500.0 {
            self.theme.error
        } else if ms > 100.0 {
            self.theme.warning
        } else {
            self.theme.success
        }
    }
}
