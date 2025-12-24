use crate::tui::Theme;
use crate::types::{ErrorKind, StatsSnapshot};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct StatusWidget<'a> {
    snapshot: &'a StatsSnapshot,
    theme: &'a Theme,
}

impl<'a> StatusWidget<'a> {
    pub fn new(snapshot: &'a StatsSnapshot, theme: &'a Theme) -> Self {
        Self { snapshot, theme }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_status_codes(frame, chunks[0]);
        self.render_errors(frame, chunks[1]);
    }

    fn render_status_codes(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" STATUS CODES ")
            .title_style(self.theme.header)
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let mut codes: Vec<_> = self.snapshot.status_codes.iter().collect();
        codes.sort_by_key(|(code, _)| *code);

        let max_count = codes.iter().map(|(_, c)| **c).max().unwrap_or(1).max(1);

        let lines: Vec<Line> = codes
            .iter()
            .take(5)
            .map(|(code, count)| {
                let bar_width = ((**count as f64 / max_count as f64) * 20.0) as usize;
                let bar: String = "█".repeat(bar_width.min(20));

                let style = if **code < 300 {
                    self.theme.success
                } else if **code < 400 {
                    self.theme.normal
                } else if **code < 500 {
                    self.theme.warning
                } else {
                    self.theme.error
                };

                Line::from(vec![
                    Span::styled(format!("{:>3}  ", code), style),
                    Span::styled(bar, self.theme.bar_filled),
                    Span::raw("  "),
                    Span::styled(format!("{}", count), self.theme.muted),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_errors(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" ERRORS ")
            .title_style(self.theme.header)
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let error_order = [
            ErrorKind::Timeout,
            ErrorKind::Connect,
            ErrorKind::Dns,
            ErrorKind::Tls,
            ErrorKind::Refused,
            ErrorKind::Reset,
            ErrorKind::Http,
            ErrorKind::Body,
            ErrorKind::Other,
        ];

        let lines: Vec<Line> = error_order
            .iter()
            .filter_map(|kind| {
                self.snapshot.errors.get(kind).map(|count| {
                    Line::from(vec![
                        Span::styled(format!("{:<10} ", kind.as_str()), self.theme.error),
                        Span::styled(format!("{}", count), self.theme.normal),
                    ])
                })
            })
            .take(5)
            .collect();

        let lines = if lines.is_empty() {
            vec![Line::from(Span::styled("No errors", self.theme.success))]
        } else {
            lines
        };

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}
