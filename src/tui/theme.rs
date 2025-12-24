use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Earth,      // Default dark theme
    Namek,      // Green planet with blue water
    Vegeta,     // Red/orange destroyed planet
    TimeChamber, // Hyperbolic Time Chamber - white void
    Tournament, // World Martial Arts Tournament - gold/purple
    Frieza,     // Frieza Force - purple/pink
}

impl ThemeMode {
    pub fn cycle(&self) -> Self {
        match self {
            ThemeMode::Earth => ThemeMode::Namek,
            ThemeMode::Namek => ThemeMode::Vegeta,
            ThemeMode::Vegeta => ThemeMode::TimeChamber,
            ThemeMode::TimeChamber => ThemeMode::Tournament,
            ThemeMode::Tournament => ThemeMode::Frieza,
            ThemeMode::Frieza => ThemeMode::Earth,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::Earth => "Earth",
            ThemeMode::Namek => "Namek",
            ThemeMode::Vegeta => "Planet Vegeta",
            ThemeMode::TimeChamber => "Time Chamber",
            ThemeMode::Tournament => "Tournament",
            ThemeMode::Frieza => "Frieza Force",
        }
    }
}

#[allow(dead_code)]
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
    pub background: Color,
}

impl Theme {
    /// Earth - Default dark theme (Goku's home)
    pub fn earth() -> Self {
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
            background: Color::Reset,
        }
    }

    /// Namek - Green skies, blue water, home of the Dragon Balls
    pub fn namek() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Rgb(0, 255, 127)) // Spring green
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(64, 224, 208)) // Turquoise
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Rgb(144, 238, 144)), // Light green
            highlight: Style::default()
                .fg(Color::Rgb(0, 255, 255)) // Cyan
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(50, 205, 50)), // Lime green
            warning: Style::default().fg(Color::Rgb(173, 255, 47)), // Green yellow
            error: Style::default().fg(Color::Rgb(255, 99, 71)), // Tomato
            muted: Style::default().fg(Color::Rgb(85, 107, 47)), // Dark olive
            bar_filled: Style::default().fg(Color::Rgb(0, 255, 127)),
            bar_empty: Style::default().fg(Color::Rgb(47, 79, 79)), // Dark slate
            border: Style::default().fg(Color::Rgb(46, 139, 87)), // Sea green
            status_ok: Style::default().fg(Color::Rgb(0, 255, 127)),
            status_error: Style::default().fg(Color::Rgb(255, 69, 0)),
            background: Color::Reset,
        }
    }

    /// Planet Vegeta - Red/orange, the Saiyan homeworld (destroyed)
    pub fn vegeta() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Rgb(255, 69, 0)) // Red-orange
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(255, 140, 0)) // Dark orange
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Rgb(255, 160, 122)), // Light salmon
            highlight: Style::default()
                .fg(Color::Rgb(255, 215, 0)) // Gold
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(255, 165, 0)), // Orange
            warning: Style::default().fg(Color::Rgb(255, 255, 0)), // Yellow
            error: Style::default().fg(Color::Rgb(220, 20, 60)), // Crimson
            muted: Style::default().fg(Color::Rgb(139, 69, 19)), // Saddle brown
            bar_filled: Style::default().fg(Color::Rgb(255, 69, 0)),
            bar_empty: Style::default().fg(Color::Rgb(128, 0, 0)), // Maroon
            border: Style::default().fg(Color::Rgb(178, 34, 34)), // Firebrick
            status_ok: Style::default().fg(Color::Rgb(255, 140, 0)),
            status_error: Style::default().fg(Color::Rgb(220, 20, 60)),
            background: Color::Reset,
        }
    }

    /// Hyperbolic Time Chamber - White void, minimal
    pub fn time_chamber() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Rgb(70, 130, 180)) // Steel blue
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(100, 149, 237)) // Cornflower blue
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Rgb(47, 79, 79)), // Dark slate gray
            highlight: Style::default()
                .fg(Color::Rgb(30, 144, 255)) // Dodger blue
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(60, 179, 113)), // Medium sea green
            warning: Style::default().fg(Color::Rgb(218, 165, 32)), // Goldenrod
            error: Style::default().fg(Color::Rgb(178, 34, 34)), // Firebrick
            muted: Style::default().fg(Color::Rgb(169, 169, 169)), // Dark gray
            bar_filled: Style::default().fg(Color::Rgb(70, 130, 180)),
            bar_empty: Style::default().fg(Color::Rgb(211, 211, 211)), // Light gray
            border: Style::default().fg(Color::Rgb(192, 192, 192)), // Silver
            status_ok: Style::default().fg(Color::Rgb(60, 179, 113)),
            status_error: Style::default().fg(Color::Rgb(178, 34, 34)),
            background: Color::Reset,
        }
    }

    /// World Tournament - Gold and purple, martial arts arena
    pub fn tournament() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Rgb(255, 215, 0)) // Gold
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(218, 165, 32)) // Goldenrod
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Rgb(238, 232, 170)), // Pale goldenrod
            highlight: Style::default()
                .fg(Color::Rgb(255, 223, 0)) // Golden yellow
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(50, 205, 50)), // Lime green
            warning: Style::default().fg(Color::Rgb(255, 165, 0)), // Orange
            error: Style::default().fg(Color::Rgb(220, 20, 60)), // Crimson
            muted: Style::default().fg(Color::Rgb(148, 0, 211)), // Dark violet
            bar_filled: Style::default().fg(Color::Rgb(255, 215, 0)),
            bar_empty: Style::default().fg(Color::Rgb(75, 0, 130)), // Indigo
            border: Style::default().fg(Color::Rgb(138, 43, 226)), // Blue violet
            status_ok: Style::default().fg(Color::Rgb(255, 215, 0)),
            status_error: Style::default().fg(Color::Rgb(220, 20, 60)),
            background: Color::Reset,
        }
    }

    /// Frieza Force - Purple and pink, galactic empire colors
    pub fn frieza() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Rgb(186, 85, 211)) // Medium orchid
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(255, 105, 180)) // Hot pink
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Rgb(221, 160, 221)), // Plum
            highlight: Style::default()
                .fg(Color::Rgb(238, 130, 238)) // Violet
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Rgb(255, 20, 147)), // Deep pink
            warning: Style::default().fg(Color::Rgb(255, 182, 193)), // Light pink
            error: Style::default().fg(Color::Rgb(139, 0, 139)), // Dark magenta
            muted: Style::default().fg(Color::Rgb(128, 0, 128)), // Purple
            bar_filled: Style::default().fg(Color::Rgb(186, 85, 211)),
            bar_empty: Style::default().fg(Color::Rgb(72, 61, 139)), // Dark slate blue
            border: Style::default().fg(Color::Rgb(148, 0, 211)), // Dark violet
            status_ok: Style::default().fg(Color::Rgb(255, 105, 180)),
            status_error: Style::default().fg(Color::Rgb(139, 0, 139)),
            background: Color::Reset,
        }
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Earth => Self::earth(),
            ThemeMode::Namek => Self::namek(),
            ThemeMode::Vegeta => Self::vegeta(),
            ThemeMode::TimeChamber => Self::time_chamber(),
            ThemeMode::Tournament => Self::tournament(),
            ThemeMode::Frieza => Self::frieza(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::earth()
    }
}
