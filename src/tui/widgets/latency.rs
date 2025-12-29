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
        // Check if this is a WebSocket test
        if self.snapshot.is_websocket {
            return self.render_websocket(frame, area);
        }

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

    fn render_websocket(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" WS MESSAGE LATENCY (ms) ")
            .title_style(self.theme.header)
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let p50 = self.snapshot.ws_latency_p50_us;
        let p95 = self.snapshot.ws_latency_p95_us;
        let p99 = self.snapshot.ws_latency_p99_us;
        let connect_mean = self.snapshot.ws_connect_time_mean_us;
        let connect_p99 = self.snapshot.ws_connect_time_p99_us;

        let max_latency = p99.max(1) as f64;

        let lines = vec![
            {
                let ms = p50 as f64 / 1000.0;
                let bar_width = ((p50 as f64 / max_latency) * 15.0) as usize;
                let bar: String = "█".repeat(bar_width.min(15));
                Line::from(vec![
                    Span::styled(" p50: ", self.theme.normal),
                    Span::styled(format!("{:>6.1}", ms), self.latency_style(ms)),
                    Span::raw("  "),
                    Span::styled(bar, self.theme.bar_filled),
                ])
            },
            {
                let ms = p95 as f64 / 1000.0;
                let bar_width = ((p95 as f64 / max_latency) * 15.0) as usize;
                let bar: String = "█".repeat(bar_width.min(15));
                Line::from(vec![
                    Span::styled(" p95: ", self.theme.normal),
                    Span::styled(format!("{:>6.1}", ms), self.latency_style(ms)),
                    Span::raw("  "),
                    Span::styled(bar, self.theme.bar_filled),
                ])
            },
            {
                let ms = p99 as f64 / 1000.0;
                let bar_width = ((p99 as f64 / max_latency) * 15.0) as usize;
                let bar: String = "█".repeat(bar_width.min(15));
                Line::from(vec![
                    Span::styled(" p99: ", self.theme.normal),
                    Span::styled(format!("{:>6.1}", ms), self.latency_style(ms)),
                    Span::raw("  "),
                    Span::styled(bar, self.theme.bar_filled),
                ])
            },
            Line::from(""),
            Line::from(vec![
                Span::styled("Connect time: ", self.theme.muted),
                Span::styled(
                    format!(
                        "mean {:.1}ms  p99 {:.1}ms",
                        connect_mean / 1000.0,
                        connect_p99 as f64 / 1000.0
                    ),
                    self.theme.normal,
                ),
            ]),
        ];

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
