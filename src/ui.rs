use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tabs
            Constraint::Min(1),    // main
            Constraint::Length(1), // status
        ])
        .split(frame.area());

    draw_tabs(frame, root[0], app);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(root[1]);

    draw_file_tree(frame, main[0], app);
    draw_editor(frame, main[1], app);
    draw_status(frame, root[2], app);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles = vec![Line::from(app.editor.title())];

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .select(0);

    frame.render_widget(tabs, area);
}

fn draw_file_tree(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .file_tree
        .entries()
        .iter()
        .map(|p| {
            let display = p
                .strip_prefix(&app.root_dir)
                .ok()
                .map(|rel| rel.display().to_string())
                .unwrap_or_else(|| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?")
                        .to_string()
                });

            ListItem::new(display)
        })
        .collect();

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(if app.focus == FocusPane::FileTree {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    let mut state = ListState::default();
    state.select(Some(app.file_tree.selected_index()));

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_editor(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let lines = app.editor.lines_for_render(inner_height);

    let text: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let line_no = app.editor.scroll_y + i + 1;
            Line::from(vec![
                Span::styled(
                    format!("{:>4} ", line_no),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(line),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Editor ")
        .borders(Borders::ALL)
        .border_style(if app.focus == FocusPane::Editor {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let focus = match app.focus {
        FocusPane::FileTree => "FILES",
        FocusPane::Editor => "EDITOR",
    };

    let root = app
        .root_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".");

    let file = app
        .editor
        .file_path
        .as_ref()
        .map(|p| {
            p.strip_prefix(&app.root_dir)
                .ok()
                .map(|rel| rel.display().to_string())
                .unwrap_or_else(|| p.display().to_string())
        })
        .unwrap_or_else(|| "[no file]".to_string());

    let position = format!(
        "Ln {}, Col {}",
        app.editor.cursor_row + 1,
        app.editor.cursor_col + 1
    );

    let dirty = if app.editor.dirty { "MOD" } else { "OK" };

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", focus),
            Style::default()
                .bg(Color::Black)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("root:{} ", root),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("| "),
        Span::raw(format!("file:{} ", file)),
        Span::raw("| "),
        Span::styled(
            format!("{} ", position),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("| "),
        Span::styled(
            format!("{} ", dirty),
            if app.editor.dirty {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            },
        ),
        Span::raw("| "),
        Span::raw(format!("{} ", app.status)),
    ]);

    let paragraph =
        Paragraph::new(status).style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_widget(paragraph, area);
}
