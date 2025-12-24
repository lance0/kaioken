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
        let block = Block::default()
            .title(" LATENCY (ms) ")
            .title_style(self.theme.header)
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let max_latency = self.snapshot.latency_p999_us.max(1) as f64;

        let percentiles = [
            ("p50", self.snapshot.latency_p50_us),
            ("p90", self.snapshot.latency_p90_us),
            ("p95", self.snapshot.latency_p95_us),
            ("p99", self.snapshot.latency_p99_us),
            ("p999", self.snapshot.latency_p999_us),
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
