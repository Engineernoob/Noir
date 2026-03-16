use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_tabs(frame, root[0], app);

    if app.terminal.visible {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(root[1]);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(body[0]);

        let editor_height = top[1].height.saturating_sub(2) as usize;
        let editor_width = top[1].width.saturating_sub(7) as usize;
        app.set_editor_viewport(editor_height, editor_width);

        let terminal_rows = body[1].height.saturating_sub(2);
        let terminal_cols = body[1].width.saturating_sub(2);
        app.resize_terminal_viewport(terminal_rows, terminal_cols);

        draw_file_tree(frame, top[0], app);
        draw_editor(frame, top[1], app);
        draw_terminal(frame, body[1], app);
    } else {
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(root[1]);

        let editor_height = main[1].height.saturating_sub(2) as usize;
        let editor_width = main[1].width.saturating_sub(7) as usize;
        app.set_editor_viewport(editor_height, editor_width);

        draw_file_tree(frame, main[0], app);
        draw_editor(frame, main[1], app);
    }

    draw_status(frame, root[2], app);

    if app.palette.open {
        draw_palette(frame, centered_rect(70, 50, frame.area()), app);
    }
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = app
        .editor
        .tab_titles()
        .into_iter()
        .map(Line::from)
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .select(app.editor.active);

    frame.render_widget(tabs, area);
}

fn draw_file_tree(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .file_tree
        .entries()
        .iter()
        .map(|entry| ListItem::new(entry.display_path.clone()))
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

fn draw_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(7) as usize;

    let lines = app.editor.lines_for_render(inner_height, inner_width);
    let scroll_y = app.editor.current_buffer().scroll_y;

    let text: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let line_no = scroll_y + i + 1;
            let mut spans = vec![Span::styled(
                format!("{:>4} ", line_no),
                Style::default().fg(Color::DarkGray),
            )];

            let tokens = app.editor.syntax.highlight(&line);
            spans.extend(highlighted_spans(&line, tokens));

            Line::from(spans)
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

    if app.focus == FocusPane::Editor {
        let (cursor_y, cursor_x) = app.editor.cursor_screen_position();

        let x = area.x + 1 + 5 + cursor_x as u16;
        let y = area.y + 1 + cursor_y as u16;

        let max_x = area.x + area.width.saturating_sub(2);
        let max_y = area.y + area.height.saturating_sub(2);

        if x <= max_x && y <= max_y {
            frame.set_cursor_position((x, y));
        }
    }
}

fn token_style(kind: &str) -> Style {
    match kind {
        "comment" => Style::default().fg(Color::DarkGray),
        "string" => Style::default().fg(Color::Green),
        "type" => Style::default().fg(Color::Cyan),
        "variable" => Style::default().fg(Color::White),
        _ => Style::default().fg(Color::White),
    }
}

fn highlighted_spans(line: &str, mut tokens: Vec<(usize, usize, &'static str)>) -> Vec<Span<'static>> {
    tokens.sort_by_key(|(start, _, _)| *start);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut cursor = 0usize;

    for (start, end, kind) in tokens {
        let start = start.min(line.len());
        let end = end.min(line.len());

        if start > cursor {
            spans.push(Span::styled(
                line[cursor..start].to_string(),
                Style::default().fg(Color::White),
            ));
        }

        if end > start {
            spans.push(Span::styled(
                line[start..end].to_string(),
                token_style(kind),
            ));
            cursor = end;
        }
    }

    if cursor < line.len() {
        spans.push(Span::styled(
            line[cursor..].to_string(),
            Style::default().fg(Color::White),
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(Color::White),
        ));
    }

    spans
}

fn draw_terminal(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let lines = app.terminal.visible_lines(inner_height);

    let mut text: Vec<Line> = if lines.is_empty() {
        vec![Line::from("No output yet.")]
    } else {
        lines.into_iter().map(Line::from).collect()
    };

    for diag in &app.diagnostics {
        text.push(Line::from(format!("⚠ {}", diag)));
    }

    let block = Block::default()
        .title(" Terminal ")
        .borders(Borders::ALL)
        .border_style(if app.focus == FocusPane::Terminal {
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
        FocusPane::Palette => "PALETTE",
        FocusPane::Terminal => "TERMINAL",
    };

    let root = app
        .root_dir
        .file_name()
        .map(|s: &std::ffi::OsStr| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());

    let buf = app.editor.current_buffer();

    let file = buf
        .file_path
        .as_ref()
        .map(|p: &std::path::PathBuf| {
            p.strip_prefix(&app.root_dir)
                .ok()
                .map(|rel: &std::path::Path| rel.display().to_string())
                .unwrap_or_else(|| p.display().to_string())
        })
        .unwrap_or_else(|| "[no file]".to_string());

    let position = format!("Ln {}, Col {}", buf.cursor_row + 1, buf.cursor_col + 1);
    let dirty = if buf.dirty { "MOD" } else { "OK" };

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
        Span::styled(position, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(
            format!(" {} ", dirty),
            if buf.dirty {
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
        Span::raw(" | "),
        Span::raw(app.status.clone()),
    ]);

    let paragraph =
        Paragraph::new(status).style(Style::default().bg(Color::White).fg(Color::Black));
    frame.render_widget(paragraph, area);
}

fn draw_palette(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let input = Paragraph::new(app.palette.input.clone())
        .block(
            Block::default()
                .title(" File Search ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);

    let items: Vec<ListItem> = if app.palette.results.is_empty() {
        vec![ListItem::new("No matches")]
    } else {
        app.palette
            .results
            .iter()
            .map(|result| ListItem::new(result.clone()))
            .collect()
    };

    let results = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Results "))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    let mut state = ListState::default();
    if !app.palette.results.is_empty() {
        state.select(Some(app.palette.selected));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(input, sections[0]);
    frame.render_stateful_widget(results, sections[1], &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
