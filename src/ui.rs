use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::{
    app::{App, FocusPane},
    lsp::DiagnosticSeverity,
    palette::PaletteMode,
    search::SearchResult,
    theme::Theme,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let status_height = if app.show_status_bar { 1 } else { 0 };
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(status_height),
        ])
        .split(frame.area());

    draw_tabs(frame, root[0], app);

    // Bottom pane is visible if either terminal or diagnostics is open.
    let show_bottom = app.terminal.visible || app.diagnostics_open;

    if show_bottom {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(root[1]);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(body[0]);

        let editor_height = top[1].height.saturating_sub(2) as usize;
        let editor_width = editor_content_width(top[1], app.show_line_numbers);
        app.set_editor_viewport(editor_height, editor_width);

        draw_file_tree(frame, top[0], app);
        draw_editor(frame, top[1], app);

        if app.diagnostics_open {
            draw_diagnostics(frame, body[1], app);
        } else {
            let terminal_rows = body[1].height.saturating_sub(2);
            let terminal_cols = body[1].width.saturating_sub(2);
            app.resize_terminal_viewport(terminal_rows, terminal_cols);
            draw_terminal(frame, body[1], app);
        }
    } else {
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(root[1]);

        let editor_height = main[1].height.saturating_sub(2) as usize;
        let editor_width = editor_content_width(main[1], app.show_line_numbers);
        app.set_editor_viewport(editor_height, editor_width);

        draw_file_tree(frame, main[0], app);
        draw_editor(frame, main[1], app);
    }

    if app.show_status_bar {
        draw_status(frame, root[2], app);
    }

    if app.palette.open {
        draw_palette(frame, centered_rect(70, 50, frame.area()), app);
    }

    if app.search.open {
        draw_search(frame, centered_rect(80, 70, frame.area()), app);
    }

    if app.hover_visible {
        draw_hover(frame, centered_rect(60, 40, frame.area()), app);
    }

    if app.prompt.open {
        draw_prompt(frame, centered_rect(60, 18, frame.area()), app);
    }

    if app.keybinding_help_open {
        draw_keybinding_help(frame, centered_rect(70, 70, frame.area()), app);
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
                .fg(app.theme.accent)
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
        .map(|entry| {
            let indent = "  ".repeat(entry.depth);

            let (icon, label, style) = if entry.is_dir {
                let icon = if entry.expanded { "▼ " } else { "▶ " };
                let label = format!("{}/", entry.name);
                let style = Style::default()
                    .fg(app.theme.accent_alt)
                    .add_modifier(Modifier::BOLD);
                (icon, label, style)
            } else {
                ("  ", entry.name.clone(), Style::default().fg(app.theme.text))
            };

            ListItem::new(Span::styled(
                format!("{indent}{icon}{label}"),
                style,
            ))
        })
        .collect();

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(if app.focus == FocusPane::FileTree {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default()
        });

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(app.theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("›");

    let mut state = ListState::default();
    state.select(Some(app.file_tree.selected_index()));

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_editor(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = editor_content_width(area, app.show_line_numbers);
    let gutter_width = editor_gutter_width(app.show_line_numbers) as u16;

    let lines = app.editor.lines_for_render(inner_height, inner_width);
    let scroll_y = app.editor.current_buffer().scroll_y;

    let text: Vec<Line> = lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let line_no = scroll_y + i + 1;
            let mut spans = Vec::new();

            if app.show_line_numbers {
                spans.push(Span::styled(
                    format!("{:>4} ", line_no),
                    Style::default().fg(app.theme.muted),
                ));
            }

            let tokens = app.editor.syntax.highlight(&line);
            spans.extend(highlighted_spans(&line, tokens, &app.theme));

            Line::from(spans)
        })
        .collect();

    let block = Block::default()
        .title(" Editor ")
        .borders(Borders::ALL)
        .border_style(if app.focus == FocusPane::Editor {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default()
        });

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);

    if app.focus == FocusPane::Editor {
        let (cursor_y, cursor_x) = app.editor.cursor_screen_position();

        let x = area.x + 1 + gutter_width + cursor_x as u16;
        let y = area.y + 1 + cursor_y as u16;

        let max_x = area.x + area.width.saturating_sub(2);
        let max_y = area.y + area.height.saturating_sub(2);

        if x <= max_x && y <= max_y {
            frame.set_cursor_position((x, y));
        }
    }
}

fn editor_gutter_width(show_line_numbers: bool) -> usize {
    if show_line_numbers { 5 } else { 0 }
}

fn editor_content_width(area: Rect, show_line_numbers: bool) -> usize {
    area.width
        .saturating_sub(2 + editor_gutter_width(show_line_numbers) as u16) as usize
}

fn draw_terminal(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let lines = app.terminal.visible_lines(inner_height);

    let text: Vec<Line> = if lines.is_empty() {
        vec![Line::from("No output yet.")]
    } else {
        lines.into_iter().map(Line::from).collect()
    };

    let block = Block::default()
        .title(" Terminal ")
        .borders(Borders::ALL)
        .border_style(if app.focus == FocusPane::Terminal {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default()
        });

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_diagnostics(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == FocusPane::Diagnostics;
    let error_count = app.diagnostic_error_count();
    let warning_count = app.diagnostic_warning_count();

    let title = if app.diagnostics_entries.is_empty() {
        " Diagnostics — no issues ".to_string()
    } else {
        let mut parts: Vec<String> = Vec::new();
        if error_count > 0 {
            parts.push(format!(
                "{} error{}",
                error_count,
                if error_count == 1 { "" } else { "s" }
            ));
        }
        if warning_count > 0 {
            parts.push(format!(
                "{} warning{}",
                warning_count,
                if warning_count == 1 { "" } else { "s" }
            ));
        }
        let other = app.diagnostics_entries.len() - error_count - warning_count;
        if other > 0 {
            parts.push(format!("{} hint{}", other, if other == 1 { "" } else { "s" }));
        }
        format!(" Diagnostics — {} ", parts.join(", "))
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = if app.diagnostics_entries.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No diagnostics",
            Style::default().fg(app.theme.success_bg),
        )]))]
    } else {
        app.diagnostics_entries
            .iter()
            .map(|entry| {
                let (badge, badge_style) = severity_badge(entry.severity, &app.theme);

                let rel_path = entry
                    .path
                    .strip_prefix(&app.root_dir)
                    .ok()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| entry.path.display().to_string());

                let location = format!("{}:{}", rel_path, entry.line + 1);
                let msg = truncate_str(&entry.message, 55);

                ListItem::new(Line::from(vec![
                    Span::styled(badge, badge_style),
                    Span::raw(" "),
                    Span::styled(
                        format!("{location:<32}"),
                        Style::default()
                            .fg(app.theme.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(msg, Style::default().fg(app.theme.muted)),
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(app.theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("›");

    let mut state = ListState::default();
    if !app.diagnostics_entries.is_empty() {
        state.select(Some(app.diagnostics_selected));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_search(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Input box
    let query_display = format!("{}_", app.search.query);
    let input = Paragraph::new(query_display)
        .block(
            Block::default()
                .title(" Text Search (Ctrl+F) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(Clear, area);
    frame.render_widget(input, sections[0]);

    // Results list
    let result_count = app.search.results.len();
    let title = if app.search.query.is_empty() {
        " Results ".to_string()
    } else if result_count == 0 {
        " Results — no matches ".to_string()
    } else if result_count >= 200 {
        format!(" Results — 200+ matches (showing first 200) ")
    } else {
        format!(" Results — {result_count} match{} ", if result_count == 1 { "" } else { "es" })
    };

    let items: Vec<ListItem> = if app.search.query.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  Start typing to search…",
            Style::default().fg(app.theme.muted),
        )]))]
    } else if app.search.results.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No matches found",
            Style::default().fg(app.theme.muted),
        )]))]
    } else {
        app.search
            .results
            .iter()
            .map(|r| search_result_item(r, &app.root_dir, &app.theme))
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    let mut state = ListState::default();
    if !app.search.results.is_empty() {
        state.select(Some(app.search.selected));
    }

    frame.render_stateful_widget(list, sections[1], &mut state);
}

fn search_result_item(
    result: &SearchResult,
    root: &std::path::Path,
    theme: &Theme,
) -> ListItem<'static> {
    let rel = result
        .path
        .strip_prefix(root)
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| result.path.display().to_string());

    let location = format!("{}:{}", rel, result.line + 1);
    let snippet = truncate_str(&result.snippet, 60);

    ListItem::new(Line::from(vec![
        Span::styled(
            format!("{location:<40}"),
            Style::default()
                .fg(theme.accent_alt)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(snippet, Style::default().fg(theme.muted)),
    ]))
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let focus_label = match app.focus {
        FocusPane::FileTree => "FILES",
        FocusPane::Editor => "EDITOR",
        FocusPane::Palette => "PALETTE",
        FocusPane::Search => "SEARCH",
        FocusPane::Terminal => "TERMINAL",
        FocusPane::Diagnostics => "DIAG",
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

    let error_count = app.diagnostic_error_count();
    let warning_count = app.diagnostic_warning_count();

    let mut spans = vec![
        Span::styled(
            format!(" {} ", focus_label),
            Style::default()
                .bg(app.theme.status_label_bg)
                .fg(app.theme.status_label_fg)
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
                    .fg(app.theme.warning_fg)
                    .bg(app.theme.warning_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(app.theme.success_fg)
                    .bg(app.theme.success_bg)
                    .add_modifier(Modifier::BOLD)
            },
        ),
        Span::raw(" "),
    ];

    // Diagnostic counts — only shown when LSP is active.
    if app.lsp.is_some() {
        if error_count > 0 {
            spans.push(Span::styled(
                format!(" ✗{} ", error_count),
                Style::default()
                    .fg(app.theme.error_fg)
                    .bg(app.theme.error_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if warning_count > 0 {
            spans.push(Span::styled(
                format!(" ⚠{} ", warning_count),
                Style::default()
                    .fg(app.theme.warning_fg)
                    .bg(app.theme.warning_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if error_count == 0 && warning_count == 0 {
            spans.push(Span::styled(
                " ✓ ",
                Style::default()
                    .fg(app.theme.success_fg)
                    .bg(app.theme.success_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::raw(" "));
    }

    spans.push(Span::raw("| "));
    spans.push(Span::raw(app.status.clone()));

    let status = Line::from(spans);
    let paragraph = Paragraph::new(status).style(
        Style::default()
            .bg(app.theme.status_bg)
            .fg(app.theme.status_fg),
    );
    frame.render_widget(paragraph, area);
}

fn draw_hover(frame: &mut Frame, area: Rect, app: &App) {
    let contents = app.hover.as_deref().unwrap_or("No hover information.");
    let hover = Paragraph::new(contents)
        .block(
            Block::default()
                .title(" Hover ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(hover, area);
}

fn draw_palette(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let (title, prompt) = match app.palette.mode {
        PaletteMode::File => (" File Search  (type > for commands) ", String::new()),
        PaletteMode::Command => (" Commands  (Backspace to return) ", "> ".to_string()),
    };

    let input_display = format!("{}{}_", prompt, app.palette.input);
    let input = Paragraph::new(input_display)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .alignment(Alignment::Left);

    let items: Vec<ListItem> = if app.palette.results.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No matches",
            Style::default().fg(app.theme.muted),
        ))]
    } else {
        match app.palette.mode {
            PaletteMode::File => app
                .palette
                .results
                .iter()
                .map(|result| ListItem::new(result.clone()))
                .collect(),
            PaletteMode::Command => app
                .command_results
                .iter()
                .map(|command| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{:<28}", command.title),
                            Style::default()
                                .fg(app.theme.text)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&command.description, Style::default().fg(app.theme.muted)),
                    ]))
                })
                .collect(),
        }
    };

    let results_title = match app.palette.mode {
        PaletteMode::File => " Files ",
        PaletteMode::Command => " Commands ",
    };

    let results = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(results_title),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.selection_bg)
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

fn draw_prompt(frame: &mut Frame, area: Rect, app: &App) {
    let Some(kind) = app.prompt.kind() else { return };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(2), Constraint::Length(1)])
        .split(area);

    let input = Paragraph::new(format!("{}_", app.prompt.input))
        .block(
            Block::default()
                .title(kind.title())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .alignment(Alignment::Left);

    let hint = Paragraph::new(kind.hint()).style(Style::default().fg(app.theme.muted));
    let footer = Paragraph::new("[Enter] submit  [Esc] close")
        .style(Style::default().fg(app.theme.muted))
        .alignment(Alignment::Right);

    frame.render_widget(Clear, area);
    frame.render_widget(input, sections[0]);
    frame.render_widget(hint, sections[1]);
    frame.render_widget(footer, sections[2]);
}

fn draw_keybinding_help(frame: &mut Frame, area: Rect, app: &App) {
    let entries = app.keybindings.help_entries();
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let inner_height = sections[0].height.saturating_sub(2) as usize;
    let start = app
        .keybinding_help_selected
        .saturating_sub(inner_height.saturating_sub(1));
    let end = (start + inner_height).min(entries.len());

    let items: Vec<ListItem> = entries[start..end]
        .iter()
        .map(|entry| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<12}", entry.context),
                    Style::default().fg(app.theme.accent_alt),
                ),
                Span::styled(
                    format!("{:<14}", entry.shortcut),
                    Style::default()
                        .fg(app.theme.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(entry.description, Style::default().fg(app.theme.muted)),
            ]))
        })
        .collect();

    let help = List::new(items)
        .block(
            Block::default()
                .title(" Keybindings ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(app.keybinding_help_selected.saturating_sub(start)));
    }

    let footer = Paragraph::new("[↑↓/PgUp/PgDn] scroll  [Esc] close")
        .style(Style::default().fg(app.theme.muted))
        .alignment(Alignment::Right);

    frame.render_widget(Clear, area);
    frame.render_stateful_widget(help, sections[0], &mut state);
    frame.render_widget(footer, sections[1]);
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn token_style(kind: &str, theme: &Theme) -> Style {
    match kind {
        "comment" => Style::default().fg(theme.syntax_comment),
        "string" => Style::default().fg(theme.syntax_string),
        "type" => Style::default().fg(theme.syntax_type),
        "variable" => Style::default().fg(theme.syntax_variable),
        _ => Style::default().fg(theme.text),
    }
}

fn highlighted_spans(
    line: &str,
    mut tokens: Vec<(usize, usize, &'static str)>,
    theme: &Theme,
) -> Vec<Span<'static>> {
    tokens.sort_by_key(|(start, _, _)| *start);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut cursor = 0usize;

    for (start, end, kind) in tokens {
        let start = start.min(line.len());
        let end = end.min(line.len());

        if start > cursor {
            spans.push(Span::styled(
                line[cursor..start].to_string(),
                Style::default().fg(theme.text),
            ));
        }

        if end > start {
            spans.push(Span::styled(
                line[start..end].to_string(),
                token_style(kind, theme),
            ));
            cursor = end;
        }
    }

    if cursor < line.len() {
        spans.push(Span::styled(
            line[cursor..].to_string(),
            Style::default().fg(theme.text),
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(theme.text),
        ));
    }

    spans
}

/// Returns a short fixed-width badge string and its style for a diagnostic severity.
fn severity_badge(severity: Option<DiagnosticSeverity>, theme: &Theme) -> (&'static str, Style) {
    match severity {
        Some(DiagnosticSeverity::ERROR) => (
            " ERR ",
            Style::default()
                .fg(theme.error_fg)
                .bg(theme.error_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Some(DiagnosticSeverity::WARNING) => (
            " WRN ",
            Style::default()
                .fg(theme.warning_fg)
                .bg(theme.warning_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Some(DiagnosticSeverity::INFORMATION) => (
            " INF ",
            Style::default().fg(theme.info_fg).bg(theme.info_bg),
        ),
        Some(DiagnosticSeverity::HINT) => (
            " HNT ",
            Style::default().fg(theme.status_fg).bg(theme.muted),
        ),
        _ => ("     ", Style::default()),
    }
}

/// Truncate to `max_chars` characters, appending `…` if the string was cut.
fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
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
