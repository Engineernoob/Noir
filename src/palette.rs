use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

#[derive(Default)]
pub struct CommandPalette {
    pub open: bool,
    pub input: String,
    pub selected: usize,
    pub results: Vec<String>,
}

impl CommandPalette {
    pub fn open(&mut self) {
        self.open = true;
        self.input.clear();
        self.selected = 0;
        self.results.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.input.clear();
        self.selected = 0;
        self.results.clear();
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

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