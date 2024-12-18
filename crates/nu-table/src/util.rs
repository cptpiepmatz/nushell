use nu_color_config::StyleComputer;
use tabled::{
    builder::Builder,
    grid::{
        ansi::{ANSIBuf, ANSIStr},
        records::vec_records::Text,
        util::string::get_text_width,
    },
    settings::{width::Truncate, Color, Modify, Padding, Style, Width},
};

use crate::common::get_leading_trailing_space_style;

pub fn string_width(text: &str) -> usize {
    get_text_width(text)
}

pub fn string_wrap(text: &str, width: usize, keep_words: bool) -> String {
    // todo: change me...
    //
    // well... it's not efficient to build a table to wrap a string,
    // but ... it's better than a copy paste (is it?)

    if text.is_empty() {
        return String::new();
    }

    let wrap = if keep_words {
        Width::wrap(width).keep_words(true)
    } else {
        Width::wrap(width)
    };

    Builder::from_iter([[text]])
        .build()
        .with(Style::empty())
        .with(Padding::zero())
        .with(Modify::new((0, 0)).with(wrap))
        .to_string()
}

pub fn string_truncate(text: &str, width: usize) -> String {
    // todo: change me...

    let line = match text.lines().next() {
        Some(first_line) => first_line,
        None => return String::new(),
    };

    Truncate::truncate(line, width).into_owned()
}

pub fn clean_charset(text: &str) -> String {
    // todo: optimize, I bet it can be done in 1 path
    text.replace('\t', "    ").replace('\r', "")
}

pub fn colorize_space(data: &mut [Vec<Text<String>>], style_computer: &StyleComputer<'_>) {
    if let Some(style) = get_leading_trailing_space_style(style_computer).color_style {
        let style = ANSIBuf::from(convert_style(style));
        let style = style.as_ref();
        colorize_lead_trail_space(data, Some(style), Some(style));
    }
}

pub fn colorize_space_str(text: &mut String, style_computer: &StyleComputer<'_>) {
    if let Some(style) = get_leading_trailing_space_style(style_computer).color_style {
        let style = ANSIBuf::from(convert_style(style));
        let style = style.as_ref();
        *text = colorize_space_one(text, Some(style), Some(style));
    }
}

fn colorize_lead_trail_space(
    data: &mut [Vec<Text<String>>],
    lead: Option<ANSIStr<'_>>,
    trail: Option<ANSIStr<'_>>,
) {
    if lead.is_none() && trail.is_none() {
        return;
    }

    for row in data.iter_mut() {
        for cell in row {
            let buf = colorize_space_one(cell.as_ref(), lead, trail);
            *cell = Text::new(buf);
        }
    }
}

fn colorize_space_one(text: &str, lead: Option<ANSIStr<'_>>, trail: Option<ANSIStr<'_>>) -> String {
    use fancy_regex::Captures;
    use fancy_regex::Regex;
    use std::sync::LazyLock;

    static RE_LEADING: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)(?P<beginsp>^\s+)").expect("error with leading space regex")
    });
    static RE_TRAILING: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)(?P<endsp>\s+$)").expect("error with trailing space regex")
    });

    let mut buf = text.to_owned();

    if let Some(color) = &lead {
        buf = RE_LEADING
            .replace_all(&buf, |cap: &Captures| {
                let spaces = cap.get(1).expect("valid").as_str();
                format!("{}{}{}", color.get_prefix(), spaces, color.get_suffix())
            })
            .into_owned();
    }

    if let Some(color) = &trail {
        buf = RE_TRAILING
            .replace_all(&buf, |cap: &Captures| {
                let spaces = cap.get(1).expect("valid").as_str();
                format!("{}{}{}", color.get_prefix(), spaces, color.get_suffix())
            })
            .into_owned();
    }

    buf
}

pub fn convert_style(style: nu_ansi_term::Style) -> Color {
    Color::new(style.prefix().to_string(), style.suffix().to_string())
}
