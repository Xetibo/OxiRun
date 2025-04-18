use std::os::unix::process::CommandExt;
use std::process::Command;

use freedesktop_desktop_entry::{default_paths, get_languages_from_env, Iter };
use iced::keyboard::key::Named;
use iced::widget::container::Style;
use iced::widget::{column, container, row, text, Column};
use iced::{event, Alignment, Element, Renderer, Subscription, Task, Theme};
use oxiced::theme::get_theme;
use oxiced::widgets::common::darken_color;
use oxiced::widgets::oxi_button::{self, ButtonVariant};
use oxiced::widgets::oxi_text_input::text_input;

use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;

pub fn main() -> Result<(), iced_layershell::Error> {
    let settings = Settings {
        layer_settings: LayerShellSettings {
            size: Some((600, 600)),
            exclusive_zone: 0,
            anchor: Anchor::Left | Anchor::Right,
            layer: Layer::Overlay,
            margin: (100, 100, 100, 100),
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    };
    OxiRun::run(settings)
}

struct OxiRun {
    theme: Theme,
    filter_text: String,
    applications: Vec<EntryInfo>
}

impl Default for OxiRun {
    fn default() -> Self {
        Self {
            theme: get_theme(),
            filter_text: "".into(),
            applications: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SetFilterText(String),
    Exit,
    LaunchEntry(EntryInfo),
}

impl TryInto<LayershellCustomActions> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayershellCustomActions, Self::Error> {
        Err(self)
    }
}

fn box_style(theme: &Theme) -> Style {
    let palette = theme.extended_palette();
    Style {
        background: Some(iced::Background::Color(darken_color(
            palette.background.base.color,
        ))),
        border: iced::border::rounded(10),
        ..container::rounded_box(theme)
    }
}

fn wrap_in_rounded_box<'a>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message> {
    container(content)
        .style(box_style)
        .align_x(Alignment::Center)
        .padding(50)
        .max_width(550)
        .into()
}

#[derive(Debug, Clone)]
struct EntryInfo {
    pub name: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub exec: String
}

fn fetch_entries() -> Vec<EntryInfo> {
    let locales = get_languages_from_env();

    let entries = Iter::new(default_paths())
        .entries(Some(&locales))
        .filter_map(|entry| {
            let name = entry.name(&locales).map(String::from)?;
            let icon = entry.icon().map(String::from);
            let categories = entry.categories().unwrap_or(Vec::new()).into_iter().map(String::from).collect();
            let exec = entry.exec().map(String::from)?;
            Some(EntryInfo{ name, icon, categories, exec })
        })
        .collect::<Vec<_>>();
    entries
}

fn create_entry_card<'a>(entry : EntryInfo) -> Element<'a, Message> {
    let btn = oxi_button::button(text(entry.name.clone()), ButtonVariant::Primary).on_press(Message::LaunchEntry(entry));
    row!(btn).into()
}

impl Application for OxiRun {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let applications = fetch_entries();
        (
            Self {
                applications,
                ..Default::default()
            },
            iced::widget::text_input::focus("search_box"),
        )
    }

    fn namespace(&self) -> String {
        String::from("OxiRun")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetFilterText(value) => {
                self.filter_text = value;
                Task::none()
            }
            Message::Exit => std::process::exit(0),
            Message::LaunchEntry(entry) => {
                Command::new("sh").arg("-c").arg(entry.exec).exec(); // TODO: remove hack & handle Freedesktop specification
                std::process::exit(0)
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let entries = self.applications.clone().into_iter().map(|entry| create_entry_card(entry)).collect::<Vec<_>>();
        let entry_container = container(Column::from_vec(entries));
        wrap_in_rounded_box(
            column!(
                text_input(
                    "Enter text to find",
                    self.filter_text.as_str(),
                    Message::SetFilterText,
                )
                .id("search_box"),
                entry_container
            ),
        )
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        event::listen_with(|event, _status, _id| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modifiers: _,
                key: iced::keyboard::key::Key::Named(Named::Escape),
                modified_key: _,
                physical_key: _,
                location: _,
                text: _,
            }) => Some(Message::Exit),
            _ => None,
        })
    }

    // remove the annoying background color
    fn style(&self, theme: &Self::Theme) -> iced_layershell::Appearance {
        let palette = theme.extended_palette();
        iced_layershell::Appearance {
            background_color: iced::Color::TRANSPARENT,
            text_color: palette.background.base.text,
        }
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}
