use iced::{
    Alignment, Element, Length, Renderer, Theme, widget::container, widget::container::Style,
};
use oxiced::widgets::common::darken_color;

use crate::Message;

pub const _SMALL_SPACING: f32 = 5.0;
pub const MEDIUM_SPACING: f32 = 10.0;
pub const _LARGE_SPACING: f32 = 15.0;
pub const _HUGE_SPACING: f32 = 20.0;

#[derive(Debug, Clone)]
pub enum FocusDirection {
    Up,
    Down,
}

impl FocusDirection {
    pub fn add(self, rhs: usize, length: usize) -> usize {
        match self {
            FocusDirection::Up => {
                if rhs > 0 {
                    rhs - 1
                } else {
                    length - 1
                }
            }
            FocusDirection::Down => {
                if length > 0 {
                    (rhs + 1) % length
                } else {
                    0
                }
            }
        }
    }
}

fn box_style(theme: &Theme) -> Style {
    let palette = theme.extended_palette();
    Style {
        background: Some(iced::Background::Color(darken_color(
            palette.background.base.color,
        ))),
        border: iced::border::rounded(MEDIUM_SPACING),
        ..container::rounded_box(theme)
    }
}

pub fn wrap_in_rounded_box<'a>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message> {
    container(content)
        .style(box_style)
        .align_x(Alignment::Center)
        .padding(50)
        .max_width(550)
        .width(Length::Fill)
        .into()
}
