use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub title: Style,
    pub header: Style,
    pub normal: Style,
    pub highlight: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub muted: Style,
    pub bar_filled: Style,
    pub bar_empty: Style,
    pub border: Style,
    pub status_ok: Style,
    pub status_error: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::White),
            highlight: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Green),
            warning: Style::default().fg(Color::Yellow),
            error: Style::default().fg(Color::Red),
            muted: Style::default().fg(Color::DarkGray),
            bar_filled: Style::default().fg(Color::Cyan),
            bar_empty: Style::default().fg(Color::DarkGray),
            border: Style::default().fg(Color::DarkGray),
            status_ok: Style::default().fg(Color::Green),
            status_error: Style::default().fg(Color::Red),
        }
    }
}
