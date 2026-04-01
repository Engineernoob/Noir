use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_alt: Color,
    pub selection_bg: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub status_label_bg: Color,
    pub status_label_fg: Color,
    pub success_bg: Color,
    pub success_fg: Color,
    pub warning_bg: Color,
    pub warning_fg: Color,
    pub error_bg: Color,
    pub error_fg: Color,
    pub info_bg: Color,
    pub info_fg: Color,
    pub syntax_comment: Color,
    pub syntax_string: Color,
    pub syntax_type: Color,
    pub syntax_variable: Color,
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "daylight" => DAYLIGHT,
            _ => NOIR,
        }
    }

    pub fn supports_name(name: &str) -> bool {
        matches!(name.trim().to_ascii_lowercase().as_str(), "noir" | "daylight")
    }

    pub fn default_name() -> &'static str {
        "noir"
    }
}

pub const NOIR: Theme = Theme {
    text: Color::White,
    muted: Color::DarkGray,
    accent: Color::Yellow,
    accent_alt: Color::Cyan,
    selection_bg: Color::DarkGray,
    status_bg: Color::White,
    status_fg: Color::Black,
    status_label_bg: Color::Black,
    status_label_fg: Color::Yellow,
    success_bg: Color::Green,
    success_fg: Color::Black,
    warning_bg: Color::Yellow,
    warning_fg: Color::Black,
    error_bg: Color::Red,
    error_fg: Color::White,
    info_bg: Color::Cyan,
    info_fg: Color::Black,
    syntax_comment: Color::DarkGray,
    syntax_string: Color::Green,
    syntax_type: Color::Cyan,
    syntax_variable: Color::White,
};

pub const DAYLIGHT: Theme = Theme {
    text: Color::Black,
    muted: Color::Gray,
    accent: Color::Blue,
    accent_alt: Color::Magenta,
    selection_bg: Color::Rgb(210, 225, 255),
    status_bg: Color::Rgb(236, 240, 244),
    status_fg: Color::Black,
    status_label_bg: Color::Blue,
    status_label_fg: Color::White,
    success_bg: Color::Green,
    success_fg: Color::Black,
    warning_bg: Color::Yellow,
    warning_fg: Color::Black,
    error_bg: Color::Red,
    error_fg: Color::White,
    info_bg: Color::Cyan,
    info_fg: Color::Black,
    syntax_comment: Color::Gray,
    syntax_string: Color::Green,
    syntax_type: Color::Blue,
    syntax_variable: Color::Black,
};
