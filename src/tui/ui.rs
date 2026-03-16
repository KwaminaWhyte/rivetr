//! TUI rendering logic using ratatui.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
        Tabs, Wrap,
    },
    Frame,
};

use super::app::{AppState, Tab};

// ─── Colours ─────────────────────────────────────────────────────────────────

const ACCENT: Color = Color::Cyan;
const SUCCESS: Color = Color::Green;
const WARNING: Color = Color::Yellow;
const ERROR: Color = Color::Red;
const DIM: Color = Color::DarkGray;
const SELECTED_BG: Color = Color::Rgb(40, 40, 60);

// ─── Main draw fn ────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.area();

    // Outer layout: header | content | status_bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_tabs(f, state, chunks[0]);

    match state.current_tab {
        Tab::Apps => draw_apps(f, state, chunks[1]),
        Tab::Deployments => draw_deployments(f, state, chunks[1]),
        Tab::Servers => draw_servers(f, state, chunks[1]),
        Tab::Logs => draw_logs(f, state, chunks[1]),
    }

    draw_status_bar(f, state, chunks[2]);

    if state.show_help {
        draw_help_overlay(f, area);
    }
}

// ─── Tab bar ─────────────────────────────────────────────────────────────────

fn draw_tabs(f: &mut Frame, state: &AppState, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(format!(" {} ", t.title())))
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(Span::styled(
                    " Rivetr TUI ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .select(state.current_tab.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        );

    f.render_widget(tabs, area);
}

// ─── Apps tab ────────────────────────────────────────────────────────────────

fn draw_apps(f: &mut Frame, state: &AppState, area: Rect) {
    let selected_style = Style::default().bg(SELECTED_BG).fg(Color::White);
    let header_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from("Name").style(header_style),
        Cell::from("Status").style(header_style),
        Cell::from("Environment").style(header_style),
        Cell::from("Domain").style(header_style),
        Cell::from("Updated").style(header_style),
    ])
    .height(1);

    let rows: Vec<Row> = state
        .apps
        .iter()
        .map(|app| {
            let status = app.status.as_deref().unwrap_or("unknown");
            let status_style = status_style(status);
            let domain = app.domain.as_deref().unwrap_or("-");
            let env = app.environment.as_deref().unwrap_or("-");
            let updated = app
                .updated_at
                .as_deref()
                .unwrap_or("-")
                .get(..16)
                .unwrap_or("-");

            Row::new(vec![
                Cell::from(app.name.as_str()),
                Cell::from(status).style(status_style),
                Cell::from(env),
                Cell::from(domain),
                Cell::from(updated),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(12),
            Constraint::Percentage(15),
            Constraint::Percentage(28),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .title(Span::styled(
                format!(" Apps ({}) ", state.apps.len()),
                Style::default().fg(Color::White),
            )),
    )
    .row_highlight_style(selected_style);

    let mut table_state = TableState::default();
    if !state.apps.is_empty() {
        table_state.select(Some(state.app_cursor));
    }

    f.render_stateful_widget(table, area, &mut table_state);
}

// ─── Deployments tab ─────────────────────────────────────────────────────────

fn draw_deployments(f: &mut Frame, state: &AppState, area: Rect) {
    let header_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let selected_style = Style::default().bg(SELECTED_BG).fg(Color::White);

    let header = Row::new(vec![
        Cell::from("ID (short)").style(header_style),
        Cell::from("App").style(header_style),
        Cell::from("Status").style(header_style),
        Cell::from("Trigger").style(header_style),
        Cell::from("Started").style(header_style),
        Cell::from("Commit").style(header_style),
    ])
    .height(1);

    let rows: Vec<Row> = state
        .deployments
        .iter()
        .map(|dep| {
            let short_id = dep.id.get(..8).unwrap_or(&dep.id);
            let status_style = status_style(&dep.status);
            let app_name = state
                .apps
                .iter()
                .find(|a| a.id == dep.app_id)
                .map(|a| a.name.as_str())
                .unwrap_or(&dep.app_id[..8.min(dep.app_id.len())]);
            let trigger = dep.trigger.as_deref().unwrap_or("manual");
            let started = dep
                .started_at
                .as_deref()
                .unwrap_or("-")
                .get(..16)
                .unwrap_or("-");
            let commit = dep
                .commit_sha
                .as_deref()
                .map(|s| &s[..7.min(s.len())])
                .unwrap_or("-");

            Row::new(vec![
                Cell::from(short_id.to_string()),
                Cell::from(app_name.to_string()),
                Cell::from(dep.status.as_str()).style(status_style),
                Cell::from(trigger.to_string()),
                Cell::from(started.to_string()),
                Cell::from(commit.to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(12),
            Constraint::Percentage(20),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(24),
            Constraint::Percentage(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .title(Span::styled(
                format!(" Deployments (last {}) ", state.deployments.len()),
                Style::default().fg(Color::White),
            )),
    )
    .row_highlight_style(selected_style);

    let mut table_state = TableState::default();
    if !state.deployments.is_empty() {
        table_state.select(Some(state.deployment_cursor));
    }

    f.render_stateful_widget(table, area, &mut table_state);
}

// ─── Servers tab ─────────────────────────────────────────────────────────────

fn draw_servers(f: &mut Frame, state: &AppState, area: Rect) {
    let header_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let selected_style = Style::default().bg(SELECTED_BG).fg(Color::White);

    let header = Row::new(vec![
        Cell::from("Name").style(header_style),
        Cell::from("Host").style(header_style),
        Cell::from("Port").style(header_style),
        Cell::from("Status").style(header_style),
    ])
    .height(1);

    let rows: Vec<Row> = state
        .servers
        .iter()
        .map(|s| {
            let status = s.status.as_deref().unwrap_or("unknown");
            let port = s
                .port
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string());
            Row::new(vec![
                Cell::from(s.name.as_str()),
                Cell::from(s.host.as_str()),
                Cell::from(port),
                Cell::from(status).style(status_style(status)),
            ])
        })
        .collect();

    if rows.is_empty() {
        let p = Paragraph::new(
            "No servers configured (or this Rivetr version has no /api/servers endpoint).",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(" Servers "),
        )
        .style(Style::default().fg(DIM));
        f.render_widget(p, area);
        return;
    }

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(10),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .title(Span::styled(
                format!(" Servers ({}) ", state.servers.len()),
                Style::default().fg(Color::White),
            )),
    )
    .row_highlight_style(selected_style);

    let mut table_state = TableState::default();
    if !state.servers.is_empty() {
        table_state.select(Some(state.server_cursor));
    }

    f.render_stateful_widget(table, area, &mut table_state);
}

// ─── Logs tab ────────────────────────────────────────────────────────────────

fn draw_logs(f: &mut Frame, state: &AppState, area: Rect) {
    let app_name = state
        .selected_app
        .as_ref()
        .map(|a| a.name.as_str())
        .unwrap_or("(none selected)");

    let title = format!(" Logs — {} (polling every 5s) ", app_name);

    if state.selected_app.is_none() {
        let p = Paragraph::new(
            "No app selected.\n\nGo to the Apps tab, select an app with ↑/↓, then press Enter.",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(title.as_str()),
        )
        .style(Style::default().fg(DIM))
        .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = state
        .logs
        .iter()
        .skip(state.log_scroll)
        .map(|entry| {
            let level = entry.level.as_deref().unwrap_or("info");
            let ts = entry
                .timestamp
                .as_deref()
                .and_then(|t| t.get(..19))
                .unwrap_or("");
            let msg = &entry.message;
            let level_style = match level {
                "error" | "ERROR" => Style::default().fg(ERROR),
                "warn" | "WARN" | "warning" => Style::default().fg(WARNING),
                "info" | "INFO" => Style::default().fg(Color::White),
                _ => Style::default().fg(DIM),
            };

            let line = if ts.is_empty() {
                Line::from(vec![
                    Span::styled(format!("[{:5}] ", level.to_uppercase()), level_style),
                    Span::raw(msg.as_str()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(ts, Style::default().fg(DIM)),
                    Span::raw(" "),
                    Span::styled(format!("[{:5}] ", level.to_uppercase()), level_style),
                    Span::raw(msg.as_str()),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .title(title.as_str()),
        )
        .highlight_style(Style::default().bg(SELECTED_BG));

    f.render_stateful_widget(list, area, &mut list_state);
}

// ─── Status bar ──────────────────────────────────────────────────────────────

fn draw_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let (conn_symbol, conn_color) = if state.connected {
        ("● Connected", SUCCESS)
    } else {
        ("○ Disconnected", ERROR)
    };

    let keybinds = " Tab:switch  ↑↓:move  Enter:select  d:deploy  s:stop  r:restart  R:refresh  ?:help  q:quit";

    let status_msg = state.status_message.as_deref().unwrap_or("");

    let left = Line::from(vec![
        Span::styled(
            conn_symbol,
            Style::default().fg(conn_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {}", state.base_url)),
        if !status_msg.is_empty() {
            Span::styled(format!("  ⚡ {}", status_msg), Style::default().fg(WARNING))
        } else {
            Span::raw("")
        },
    ]);

    let right_text = keybinds;

    let bar_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(right_text.len() as u16 + 1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(left).style(Style::default().bg(Color::Rgb(20, 20, 35))),
        bar_area[0],
    );
    f.render_widget(
        Paragraph::new(right_text)
            .style(Style::default().fg(DIM).bg(Color::Rgb(20, 20, 35)))
            .alignment(Alignment::Right),
        bar_area[1],
    );
}

// ─── Help overlay ────────────────────────────────────────────────────────────

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled(
            " Rivetr TUI — Keyboard Shortcuts ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab / →      ", Style::default().fg(ACCENT)),
            Span::raw("Next tab"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab / ←", Style::default().fg(ACCENT)),
            Span::raw("Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / k        ", Style::default().fg(ACCENT)),
            Span::raw("Move cursor up"),
        ]),
        Line::from(vec![
            Span::styled("  ↓ / j        ", Style::default().fg(ACCENT)),
            Span::raw("Move cursor down"),
        ]),
        Line::from(vec![
            Span::styled("  Enter        ", Style::default().fg(ACCENT)),
            Span::raw("Select app → open Logs tab"),
        ]),
        Line::from(vec![
            Span::styled("  d             ", Style::default().fg(ACCENT)),
            Span::raw("Trigger deploy (Apps tab)"),
        ]),
        Line::from(vec![
            Span::styled("  s             ", Style::default().fg(ACCENT)),
            Span::raw("Stop app (Apps tab)"),
        ]),
        Line::from(vec![
            Span::styled("  r             ", Style::default().fg(ACCENT)),
            Span::raw("Restart app (Apps tab)"),
        ]),
        Line::from(vec![
            Span::styled("  R             ", Style::default().fg(ACCENT)),
            Span::raw("Force refresh now"),
        ]),
        Line::from(vec![
            Span::styled("  ?             ", Style::default().fg(ACCENT)),
            Span::raw("Toggle this help overlay"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc       ", Style::default().fg(ACCENT)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C        ", Style::default().fg(ACCENT)),
            Span::raw("Force quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close this overlay",
            Style::default().fg(DIM),
        )),
    ];

    let width = 52u16;
    let height = help_text.len() as u16 + 2;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Rgb(15, 15, 30)));

    let para = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(Clear, popup_area);
    f.render_widget(para, popup_area);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn status_style(status: &str) -> Style {
    match status.to_lowercase().as_str() {
        "running" | "success" | "succeeded" | "healthy" | "connected" | "online" => {
            Style::default().fg(SUCCESS)
        }
        "failed" | "error" | "crashed" | "offline" | "disconnected" => Style::default().fg(ERROR),
        "building" | "cloning" | "deploying" | "pending" | "starting" | "checking" => {
            Style::default().fg(WARNING)
        }
        "stopped" | "inactive" => Style::default().fg(DIM),
        _ => Style::default().fg(Color::White),
    }
}
