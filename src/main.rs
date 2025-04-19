use std::hash::{Hash, Hasher};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use freedesktop_desktop_entry::{default_paths, get_languages_from_env, Iter};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use iced::advanced::image::Id;
use iced::advanced::widget::operation::focusable::focus;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::{svg, Widget};
use iced::keyboard::key::Named;
use iced::widget::container::Style;
use iced::widget::{column, container, row, text, Button, Column, Row};
use iced::{event, Alignment, Element, Length, Renderer, Subscription, Task, Theme};
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
    applications: Vec<EntryInfo>,
    sorted_applications: Vec<EntryInfo>,
    current_focus: usize,
}

impl Default for OxiRun {
    fn default() -> Self {
        Self {
            theme: get_theme(),
            filter_text: "".into(),
            applications: Vec::new(),
            sorted_applications: Vec::new(),
            current_focus: 0,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SetFilterText(String),
    Exit,
    LaunchEntry(EntryInfo),
    LaunchFocusedEntry,
    MoveApplicationFocus,
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
        .width(Length::Fill)
        .into()
}

#[derive(Debug, Clone)]
struct EntryInfo {
    pub name: String,
    pub icon: Option<PathBuf>,
    pub categories: Vec<String>,
    pub exec: String,
}

fn fetch_entries() -> Vec<EntryInfo> {
    let locales = get_languages_from_env();

    fn get_icon_path(icon_str: &str) -> Option<PathBuf> {
        let icon_source = freedesktop_desktop_entry::IconSource::from_unknown(icon_str);
        match icon_source {
            freedesktop_desktop_entry::IconSource::Name(name) => {
                if let Some(path) = freedesktop_icons::lookup(&name).find() {
                    Some(path)
                } else {
                    None
                }
            }
            freedesktop_desktop_entry::IconSource::Path(path) => Some(path),
        }
    }

    let entries = Iter::new(default_paths())
        .entries(Some(&locales))
        .filter_map(|entry| {
            let name = entry.name(&locales).map(String::from)?;
            let icon = entry.icon().map(get_icon_path).flatten();
            let categories = entry
                .categories()
                .unwrap_or(Vec::new())
                .into_iter()
                .map(String::from)
                .collect();
            let exec = entry.exec().map(String::from)?;
            Some(EntryInfo {
                name,
                icon,
                categories,
                exec,
            })
        })
        .collect::<Vec<_>>();
    entries
}

// TODO move this to lib as it should have proper styling
struct OxiRunButton<'a> {
    button: Button<'a, Message>,
    focused: bool,
}

impl<'a> Into<Element<'a, Message>> for OxiRunButton<'a> {
    fn into(self) -> Element<'a, Message> {
        self.button.into()
    }
}

impl<'a> Focusable for OxiRunButton<'a> {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
        // TODO change state to hover
    }

    fn unfocus(&mut self) {
        self.focused = false;
        // TODO change state to hover
    }
}

impl<'a> Widget<Message, Theme, Renderer> for OxiRunButton<'a> {
    fn size(&self) -> iced::Size<Length> {
        self.button.size()
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.button.layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.button
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }
}

fn create_entry_card<'a>((index, entry): (usize, EntryInfo)) -> Element<'a, Message> {
    let icon = if let Some(icon) = &entry.icon {
        Some(iced::widget::image(icon).height(Length::Fixed(75.0)))
    } else {
        None
    };
    let content = Row::new()
        .push_maybe(icon)
        .push(container(text(entry.name.clone())).align_right(Length::Fill));
    // TODO use oxibutton directly again when possible
    let inner_button = oxi_button::button(content, ButtonVariant::Primary)
        .on_press(Message::LaunchEntry(entry))
        .width(Length::Fill)
        .height(Length::Fixed(50.0));
    let btn = OxiRunButton {
        button: inner_button,
        focused: false,
    };
    container(btn)
        .id(iced::widget::container::Id::new(format!(
            "appliation{}",
            index
        )))
        .into()
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
                sorted_applications: applications.clone(),
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
                let matcher = SkimMatcherV2::default();
                let mut sorted_applications = self.applications.clone();
                sorted_applications.sort_by(|first, second| {
                    matcher
                        .fuzzy_match(&second.name, &self.filter_text)
                        .cmp(&matcher.fuzzy_match(&first.name, &self.filter_text))
                });
                self.sorted_applications = sorted_applications;
                Task::none()
            }
            Message::Exit => std::process::exit(0),
            Message::LaunchEntry(entry) => {
                Command::new("sh").arg("-c").arg(entry.exec).exec(); // TODO: remove hack & handle Freedesktop specification
                std::process::exit(0)
            }
            Message::MoveApplicationFocus => {
                if self.current_focus < 9 {
                    // TODO make this amount configurable -> max amount of
                    // entries
                    self.current_focus += 1;
                } else {
                    self.current_focus = 0;
                }
                iced::widget::focus_next()
            }
            Message::LaunchFocusedEntry => {
                if let Some(entry) = self.sorted_applications.get(self.current_focus) {
                    Command::new("sh").arg("-c").arg(entry.exec.clone()).exec();
                    // TODO: remove hack & handle Freedesktop specification
                }
                std::process::exit(0)
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let entries = self
            .sorted_applications
            .clone()
            .into_iter()
            .take(10) // TODO make this configurable
            .enumerate()
            .map(create_entry_card)
            .collect::<Vec<_>>();
        let entry_container = Column::from_vec(entries).width(Length::Fill).spacing(10);
        wrap_in_rounded_box(
            column!(
                text_input(
                    "Enter text to find",
                    self.filter_text.as_str(),
                    Message::SetFilterText,
                )
                .id("search_box"),
                entry_container
            )
            .width(Length::Fill),
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
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modifiers: _,
                key: iced::keyboard::key::Key::Named(Named::Tab),
                modified_key: _,
                physical_key: _,
                location: _,
                text: _,
            }) => Some(Message::MoveApplicationFocus),
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modifiers: _,
                key: iced::keyboard::key::Key::Named(Named::Enter),
                modified_key: _,
                physical_key: _,
                location: _,
                text: _,
            }) => Some(Message::LaunchFocusedEntry),
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
