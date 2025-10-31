//! Main TUI application state and navigation

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use super::event::{Event, EventHandler};
use super::screens::{Screen, ScreenType};

/// Main TUI application
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    event_handler: EventHandler,
    current_screen: Box<dyn Screen>,
    should_quit: bool,
}

impl TuiApp {
    /// Create new TUI app with initial screen
    pub fn new(initial_screen: ScreenType) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let event_handler = EventHandler::new(std::time::Duration::from_millis(250));
        let current_screen = initial_screen.create()?;

        Ok(Self {
            terminal,
            event_handler,
            current_screen,
            should_quit: false,
        })
    }

    /// Run the TUI application
    pub fn run(&mut self) -> Result<()> {
        loop {
            // Draw the current screen
            self.terminal.draw(|f| {
                let area = f.size();
                self.current_screen.render(f, area)
            })?;

            // Handle events
            let event = self.event_handler.next()?;

            match event {
                Event::Key(key) => {
                    if super::event::is_ctrl_c(&key) {
                        self.should_quit = true;
                    } else {
                        let action = self.current_screen.handle_key(key)?;
                        self.handle_screen_action(action)?;
                    }
                }
                Event::Tick => {
                    self.current_screen.update()?;
                }
                Event::Resize(_, _) => {
                    // Terminal will automatically redraw on resize
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle screen navigation actions
    fn handle_screen_action(&mut self, action: super::screens::ScreenAction) -> Result<()> {
        use super::screens::ScreenAction;

        match action {
            ScreenAction::None => {}
            ScreenAction::SwitchTo(screen_type) => {
                self.current_screen = screen_type.create()?;
            }
            ScreenAction::Back => {
                // Navigate back to Dashboard by default
                self.current_screen = ScreenType::Dashboard.create()?;
            }
            ScreenAction::Quit => {
                self.should_quit = true;
            }
        }

        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
