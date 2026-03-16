#[derive(Default)]
pub struct CommandPalette {
    pub open: bool,
    pub input: String,
}

impl CommandPalette {
    pub fn open(&mut self) {
        self.open = true;
        self.input.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.input.clear();
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }
}