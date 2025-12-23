use crate::output::write_json;
use crate::tui::{ui, Flavor, Theme};
use crate::types::{LoadConfig, RunState, StatsSnapshot};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::time::Duration;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub struct App {
    config: LoadConfig,
    snapshot_rx: watch::Receiver<StatsSnapshot>,
    state_rx: watch::Receiver<RunState>,
    cancel_token: CancellationToken,
    theme: Theme,
    flavor: Flavor,
    output_path: Option<String>,
}

impl App {
    pub fn new(
        config: LoadConfig,
        snapshot_rx: watch::Receiver<StatsSnapshot>,
        state_rx: watch::Receiver<RunState>,
        cancel_token: CancellationToken,
        serious: bool,
        output_path: Option<String>,
    ) -> Self {
        Self {
            config,
            snapshot_rx,
            state_rx,
            cancel_token,
            theme: Theme::default(),
            flavor: Flavor::new(serious),
            output_path,
        }
    }

    pub async fn run(mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.event_loop(&mut terminal).await;

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            let snapshot = self.snapshot_rx.borrow().clone();
            let state = *self.state_rx.borrow();

            terminal.draw(|frame| {
                ui::render(
                    frame,
                    &snapshot,
                    state,
                    &self.config.url,
                    self.config.concurrency,
                    self.config.duration,
                    &self.theme,
                    &self.flavor,
                );
            })?;

            if state.is_terminal() {
                tokio::time::sleep(Duration::from_millis(500)).await;
                break;
            }

            tokio::select! {
                _ = interval.tick() => {}
                _ = self.cancel_token.cancelled() => {
                    break;
                }
            }

            while event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                self.cancel_token.cancel();
                            }
                            KeyCode::Char('s') => {
                                if let Some(path) = &self.output_path {
                                    let _ = write_json(&snapshot, &self.config, path);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
