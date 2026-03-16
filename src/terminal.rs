#[derive(Default)]
pub struct TerminalPane {
    pub visible: bool,
    pub lines: Vec<String>,
    pub scroll: usize,
}

impl TerminalPane {
    pub fn new() -> Self {
        Self {
            visible: true,
            lines: vec![
                "Noir terminal pane initialized.".to_string(),
                "This is a placeholder output panel for now.".to_string(),
                "Next phase: real PTY-backed terminal.".to_string(),
            ],
            scroll: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn push_line<S: Into<String>>(&mut self, line: S) {
        self.lines.push(line.into());
    }

    pub fn visible_lines(&self, height: usize) -> Vec<String> {
        let total = self.lines.len();
        if total == 0 {
            return vec![];
        }

        let start = self.scroll.min(total.saturating_sub(1));
        let end = (start + height).min(total);

        self.lines[start..end].to_vec()
    }

    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll + 1 < self.lines.len() {
            self.scroll += 1;
        }
    }
}
