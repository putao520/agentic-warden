use aiw::tui::screens::{
    dashboard::DashboardScreen, provider::ProviderScreen, status::StatusScreen, ScreenAction,
    ScreenType,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

struct TempHome {
    dir: TempDir,
    old_home: Option<std::ffi::OsString>,
    old_user: Option<std::ffi::OsString>,
}

impl TempHome {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &path);
        let old_user = std::env::var_os("USERPROFILE");
        if cfg!(windows) {
            std::env::set_var("USERPROFILE", &path);
        }
        Self {
            dir,
            old_home,
            old_user,
        }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        if let Some(old) = self.old_home.take() {
            std::env::set_var("HOME", old);
        } else {
            std::env::remove_var("HOME");
        }
        if cfg!(windows) {
            if let Some(old) = self.old_user.take() {
                std::env::set_var("USERPROFILE", old);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }
}

#[test]
fn dashboard_navigation_switches_to_provider_and_status() {
    let _home = TempHome::new();
    let mut dashboard = DashboardScreen::new().expect("dashboard screen");

    let to_provider = dashboard
        .handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
        .expect("handle key");
    assert!(matches!(
        to_provider,
        ScreenAction::SwitchTo(ScreenType::Provider)
    ));

    let to_status = dashboard
        .handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
        .expect("handle key");
    assert!(matches!(
        to_status,
        ScreenAction::SwitchTo(ScreenType::Status)
    ));
}

#[test]
fn provider_screen_back_navigation_returns_to_previous_screen() {
    let _home = TempHome::new();
    let mut provider = ProviderScreen::new().expect("provider screen");

    let action = provider
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .expect("handle key");
    assert!(matches!(action, ScreenAction::Back));
}

#[test]
fn status_screen_refresh_shortcut_triggers_update_message() {
    let _home = TempHome::new();
    let mut status = StatusScreen::new().expect("status screen");

    let action = status
        .handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
        .expect("handle key");
    assert!(matches!(action, ScreenAction::None));

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| status.render(frame, frame.size()))
        .unwrap();
    let mut rendered = String::new();
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer.get(x, y).symbol());
        }
        rendered.push('\n');
    }
    assert!(
        rendered.contains("Tasks refreshed"),
        "status message not rendered:\n{rendered}"
    );
}
