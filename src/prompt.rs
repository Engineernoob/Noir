#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    GoToLine,
    CreateFile,
    EditFile,
}

impl PromptKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::GoToLine => " Go To Line ",
            Self::CreateFile => " Create File ",
            Self::EditFile => " Edit File ",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::GoToLine => "Enter line or line:column",
            Self::CreateFile => "Enter a path relative to the workspace root",
            Self::EditFile => "Enter an existing file path relative to the workspace root",
        }
    }
}

#[derive(Debug, Default)]
pub struct InputPrompt {
    pub open: bool,
    pub kind: Option<PromptKind>,
    pub input: String,
}

impl InputPrompt {
    pub fn open(&mut self, kind: PromptKind, input: String) {
        self.open = true;
        self.kind = Some(kind);
        self.input = input;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.kind = None;
        self.input.clear();
    }

    pub fn kind(&self) -> Option<PromptKind> {
        self.kind
    }
}
