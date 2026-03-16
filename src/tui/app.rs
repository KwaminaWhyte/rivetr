//! TUI application state and event loop.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use super::api::{ApiClient, App, Deployment, LogEntry, Server};
use super::ui;

// ─── Tab ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Apps,
    Deployments,
    Servers,
    Logs,
}

impl Tab {
    pub const ALL: &'static [Tab] = &[Tab::Apps, Tab::Deployments, Tab::Servers, Tab::Logs];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Apps => "Apps",
            Tab::Deployments => "Deployments",
            Tab::Servers => "Servers",
            Tab::Logs => "Logs",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

// ─── AppState ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub api: Arc<ApiClient>,
    pub current_tab: Tab,
    pub show_help: bool,
    pub status_message: Option<String>,

    // Apps tab
    pub apps: Vec<App>,
    pub app_cursor: usize,
    pub selected_app: Option<App>,

    // Deployments tab
    pub deployments: Vec<Deployment>,
    pub deployment_cursor: usize,

    // Servers tab
    pub servers: Vec<Server>,
    pub server_cursor: usize,

    // Logs tab
    pub logs: Vec<LogEntry>,
    pub log_scroll: usize,

    // Timing
    pub last_refresh: Instant,
    pub refresh_interval: Duration,

    // Connection
    pub connected: bool,
    pub base_url: String,
}

impl AppState {
    pub fn new(api: ApiClient, base_url: String) -> Self {
        Self {
            api: Arc::new(api),
            current_tab: Tab::Apps,
            show_help: false,
            status_message: None,
            apps: Vec::new(),
            app_cursor: 0,
            selected_app: None,
            deployments: Vec::new(),
            deployment_cursor: 0,
            servers: Vec::new(),
            server_cursor: 0,
            logs: Vec::new(),
            log_scroll: 0,
            last_refresh: Instant::now() - Duration::from_secs(60), // force immediate refresh
            refresh_interval: Duration::from_secs(5),
            connected: false,
            base_url,
        }
    }

    /// Refresh all data from the API.
    pub fn refresh(&mut self) {
        self.last_refresh = Instant::now();
        self.connected = self.api.ping();

        if !self.connected {
            self.status_message = Some("Cannot connect to Rivetr API".to_string());
            return;
        }

        // Clear previous status before refreshing
        self.status_message = None;

        match self.api.list_apps() {
            Ok(apps) => {
                // Keep cursor in bounds after refresh
                if self.app_cursor >= apps.len() && !apps.is_empty() {
                    self.app_cursor = apps.len() - 1;
                }
                // Update selected_app to keep it fresh
                if let Some(ref sel) = self.selected_app.clone() {
                    self.selected_app = apps.iter().find(|a| a.id == sel.id).cloned();
                }
                // Fetch recent deployments across all apps (up to 5 apps, 5 each)
                let mut all_deps: Vec<super::api::Deployment> = Vec::new();
                for app in apps.iter().take(5) {
                    if let Ok(mut deps) = self.api.list_app_deployments(&app.id, 5) {
                        all_deps.append(&mut deps);
                    }
                }
                // Sort by started_at descending and keep the most recent 20
                all_deps.sort_by(|a, b| {
                    b.started_at
                        .as_deref()
                        .unwrap_or("")
                        .cmp(a.started_at.as_deref().unwrap_or(""))
                });
                all_deps.truncate(20);
                if self.deployment_cursor >= all_deps.len() && !all_deps.is_empty() {
                    self.deployment_cursor = all_deps.len() - 1;
                }
                self.deployments = all_deps;
                self.apps = apps;
            }
            Err(e) => {
                self.status_message = Some(format!("Apps error: {}", e));
            }
        }

        match self.api.list_servers() {
            Ok(servers) => {
                if self.server_cursor >= servers.len() && !servers.is_empty() {
                    self.server_cursor = servers.len() - 1;
                }
                self.servers = servers;
            }
            Err(_) => {
                // Servers endpoint may not exist on all versions — soft fail
                self.servers = Vec::new();
            }
        }

        // If an app is selected and we're on the Logs tab, poll for logs
        if self.current_tab == Tab::Logs {
            self.refresh_logs();
        }
    }

    fn refresh_logs(&mut self) {
        let selected = match &self.selected_app {
            Some(a) => a.clone(),
            None => return,
        };

        // Find the latest deployment for this app
        let dep = self
            .deployments
            .iter()
            .find(|d| d.app_id == selected.id)
            .cloned();

        if let Some(dep) = dep {
            match self.api.fetch_logs(&selected.id, &dep.id) {
                Ok(logs) => {
                    self.logs = logs;
                }
                Err(e) => {
                    self.status_message = Some(format!("Log fetch error: {}", e));
                }
            }
        }
    }

    pub fn next_tab(&mut self) {
        let idx = (self.current_tab.index() + 1) % Tab::ALL.len();
        self.current_tab = Tab::ALL[idx];
    }

    pub fn prev_tab(&mut self) {
        let idx = if self.current_tab.index() == 0 {
            Tab::ALL.len() - 1
        } else {
            self.current_tab.index() - 1
        };
        self.current_tab = Tab::ALL[idx];
    }

    pub fn cursor_down(&mut self) {
        match self.current_tab {
            Tab::Apps => {
                if !self.apps.is_empty() {
                    self.app_cursor = (self.app_cursor + 1).min(self.apps.len() - 1);
                }
            }
            Tab::Deployments => {
                if !self.deployments.is_empty() {
                    self.deployment_cursor =
                        (self.deployment_cursor + 1).min(self.deployments.len() - 1);
                }
            }
            Tab::Servers => {
                if !self.servers.is_empty() {
                    self.server_cursor = (self.server_cursor + 1).min(self.servers.len() - 1);
                }
            }
            Tab::Logs => {
                if !self.logs.is_empty() {
                    self.log_scroll = (self.log_scroll + 1).min(self.logs.len().saturating_sub(1));
                }
            }
        }
    }

    pub fn cursor_up(&mut self) {
        match self.current_tab {
            Tab::Apps => {
                self.app_cursor = self.app_cursor.saturating_sub(1);
            }
            Tab::Deployments => {
                self.deployment_cursor = self.deployment_cursor.saturating_sub(1);
            }
            Tab::Servers => {
                self.server_cursor = self.server_cursor.saturating_sub(1);
            }
            Tab::Logs => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
        }
    }

    /// Select the app under the cursor and switch to Logs tab.
    pub fn select_app(&mut self) {
        if let Some(app) = self.apps.get(self.app_cursor) {
            self.selected_app = Some(app.clone());
            self.current_tab = Tab::Logs;
            self.logs = Vec::new();
            self.log_scroll = 0;
            self.refresh_logs();
        }
    }

    pub fn trigger_deploy(&mut self) {
        let app = match self.apps.get(self.app_cursor) {
            Some(a) => a.clone(),
            None => return,
        };
        match self.api.deploy_app(&app.id) {
            Ok(_) => {
                self.status_message = Some(format!("Deploy triggered for '{}'", app.name));
            }
            Err(e) => {
                self.status_message = Some(format!("Deploy failed: {}", e));
            }
        }
    }

    pub fn trigger_stop(&mut self) {
        let app = match self.apps.get(self.app_cursor) {
            Some(a) => a.clone(),
            None => return,
        };
        match self.api.stop_app(&app.id) {
            Ok(_) => {
                self.status_message = Some(format!("Stop requested for '{}'", app.name));
            }
            Err(e) => {
                self.status_message = Some(format!("Stop failed: {}", e));
            }
        }
    }

    pub fn trigger_restart(&mut self) {
        let app = match self.apps.get(self.app_cursor) {
            Some(a) => a.clone(),
            None => return,
        };
        match self.api.restart_app(&app.id) {
            Ok(_) => {
                self.status_message = Some(format!("Restart requested for '{}'", app.name));
            }
            Err(e) => {
                self.status_message = Some(format!("Restart failed: {}", e));
            }
        }
    }
}

// ─── Event loop ──────────────────────────────────────────────────────────────

/// Run the TUI. Blocks until the user quits.
pub fn run_tui(mut state: AppState) -> Result<()> {
    // Set up terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initial data load
    state.refresh();

    let poll_timeout = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui::draw(f, &state))?;

        // Periodic refresh
        if state.last_refresh.elapsed() >= state.refresh_interval {
            state.refresh();
        }

        // Poll for keyboard events
        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                // Ctrl-C / q always quits
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                if state.show_help {
                    // Any key closes help
                    state.show_help = false;
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('?') => state.show_help = !state.show_help,
                    KeyCode::Tab | KeyCode::Right => state.next_tab(),
                    KeyCode::BackTab | KeyCode::Left => state.prev_tab(),
                    KeyCode::Down | KeyCode::Char('j') => state.cursor_down(),
                    KeyCode::Up | KeyCode::Char('k') => state.cursor_up(),
                    KeyCode::Enter => {
                        if state.current_tab == Tab::Apps {
                            state.select_app();
                        }
                    }
                    KeyCode::Char('d') => {
                        if state.current_tab == Tab::Apps {
                            state.trigger_deploy();
                        }
                    }
                    KeyCode::Char('s') => {
                        if state.current_tab == Tab::Apps {
                            state.trigger_stop();
                        }
                    }
                    KeyCode::Char('r') => {
                        if state.current_tab == Tab::Apps {
                            state.trigger_restart();
                        }
                    }
                    KeyCode::Char('R') => {
                        // Force refresh
                        state.last_refresh =
                            Instant::now() - state.refresh_interval - Duration::from_secs(1);
                    }
                    _ => {}
                }

                // Clear status message on next keypress
                state.status_message = None;
            }
        }
    }

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    Ok(())
}
