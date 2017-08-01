//! Defines the custom theme for Cursive

use cursive::theme::{Theme, BorderStyle, Palette, Color, BaseColor};

pub const THEME: Theme = Theme {
    shadow: false,
    borders: BorderStyle::Simple,
    colors: Palette {
        background: Color::RgbLowRes(0,1,1),
        shadow: Color::Dark(BaseColor::Magenta),
        view: Color::Dark(BaseColor::White),
        primary: Color::Dark(BaseColor::Black),
        secondary: Color::Dark(BaseColor::Blue),
        tertiary: Color::Dark(BaseColor::White),
        title_primary: Color::Dark(BaseColor::Red),
        title_secondary: Color::Dark(BaseColor::Yellow),
        highlight: Color::Dark(BaseColor::Red),
        highlight_inactive: Color::Dark(BaseColor::Blue),
    },
};
