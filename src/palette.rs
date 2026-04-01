use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaletteMode {
    #[default]
    File,
    Command,
}

pub struct CommandPalette {
    pub open: bool,
    pub input: String,
    pub mode: PaletteMode,
    pub selected: usize,
    pub results: Vec<String>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            open: false,
            input: String::new(),
            mode: PaletteMode::File,
            selected: 0,
            results: Vec::new(),
        }
    }
}

impl CommandPalette {
    pub fn open(&mut self) {
        self.open = true;
        self.mode = PaletteMode::File;
        self.input.clear();
        self.selected = 0;
        self.results.clear();
    }

    pub fn open_command_mode(&mut self) {
        self.open = true;
        self.mode = PaletteMode::Command;
        self.input.clear();
        self.selected = 0;
        self.results.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.input.clear();
        self.mode = PaletteMode::File;
        self.selected = 0;
        self.results.clear();
    }
    /// Fuzzy-filter `candidates` (file names or command names) into `results`.
    pub fn update_results<I>(&mut self, candidates: I)
    where
        I: IntoIterator<Item = String>,
    {
        let matcher = SkimMatcherV2::default();

        let mut scored: Vec<(i64, String)> = candidates
            .into_iter()
            .filter_map(|candidate| {
                if self.input.is_empty() {
                    Some((0, candidate))
                } else {
                    matcher
                        .fuzzy_match(&candidate, &self.input)
                        .map(|score| (score, candidate))
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

        self.results = scored.into_iter().map(|(_, s)| s).take(20).collect();

        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn selected_result(&self) -> Option<&str> {
        self.results.get(self.selected).map(String::as_str)
    }
}
