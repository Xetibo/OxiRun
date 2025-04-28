use std::sync::Arc;

use applications::{
    EntryInfo, ScoredEntryInfo, create_entry_card, fetch_entries, run_command, sort_appliations,
};
use config::{Config, get_config};
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::{Column, column};
use iced::{Element, Length, Subscription, Task, Theme, event};
use oxiced::theme::get_theme;
use oxiced::widgets::oxi_text_input::text_input;

use iced_layershell::Application;
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use utils::{FocusDirection, MEDIUM_SPACING, wrap_in_rounded_box};

mod applications;
mod config;
mod utils;

// TODO make this configurable
const ICON_SIZE: f32 = 50.0;
const SORT_THRESHOLD: i64 = 25;
const SCALE_FACTOR: f64 = 1.0;
const WINDOW_SIZE: (u32, u32) = (600, 600);
const WINDOW_MARGINS: (i32, i32, i32, i32) = (100, 100, 100, 100);
const WINDOW_LAYER: Layer = Layer::Overlay;
const WINDOW_KEYBAORD_MODE: KeyboardInteractivity = KeyboardInteractivity::Exclusive;

pub fn main() -> Result<(), iced_layershell::Error> {
    let settings = Settings {
        layer_settings: LayerShellSettings {
            size: Some(WINDOW_SIZE),
            exclusive_zone: 0,
            anchor: Anchor::Left | Anchor::Right,
            layer: WINDOW_LAYER,
            margin: WINDOW_MARGINS,
            keyboard_interactivity: WINDOW_KEYBAORD_MODE,
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
    sorted_applications: Vec<ScoredEntryInfo>,
    fuzzy_matcher: Arc<SkimMatcherV2>,
    current_focus: usize,
    config: Config,
}

impl Default for OxiRun {
    fn default() -> Self {
        Self {
            theme: get_theme(),
            filter_text: "".into(),
            applications: Vec::new(),
            sorted_applications: Vec::new(),
            fuzzy_matcher: Arc::new(SkimMatcherV2::default()), // TODO should this be async as well?
            current_focus: 0,
            config: Config::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SetFilterText(String),
    Exit,
    LaunchEntry(EntryInfo),
    LaunchFocusedEntry,
    MoveApplicationFocus(FocusDirection),
    ReceiveEntries(Vec<EntryInfo>),
    ReceiveSortedEntries(Vec<ScoredEntryInfo>),
}

impl TryInto<LayershellCustomActions> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayershellCustomActions, Self::Error> {
        Err(self)
    }
}

impl Application for OxiRun {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let config = get_config();
        (
            Self {
                config: config.clone(),
                ..Default::default()
            },
            Task::batch([
                iced::widget::text_input::focus("search_box"),
                Task::future(fetch_entries(config)),
            ]),
        )
    }

    fn namespace(&self) -> String {
        String::from("OxiRun")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetFilterText(value) => {
                self.filter_text = value;
                let entries = self.applications.clone();
                let filter_text = self.filter_text.clone();
                let matcher = self.fuzzy_matcher.clone();
                Task::future(sort_appliations(entries, filter_text, matcher))
            }
            Message::ReceiveSortedEntries(sorted_entries) => {
                self.sorted_applications = sorted_entries;
                Task::none()
            }
            Message::Exit => std::process::exit(0),
            Message::LaunchEntry(entry) => {
                run_command(&entry.exec);
                std::process::exit(0)
            }
            Message::MoveApplicationFocus(direction) => {
                self.current_focus =
                    direction.add(self.current_focus, self.sorted_applications.len());
                iced::widget::focus_next()
            }
            Message::LaunchFocusedEntry => {
                if let Some(scored_entry) = self.sorted_applications.get(self.current_focus) {
                    run_command(&scored_entry.entry.exec);
                }
                std::process::exit(0)
            }
            Message::ReceiveEntries(entry_infos) => {
                self.applications = entry_infos.clone();
                let filter_text = self.filter_text.clone();
                let matcher = self.fuzzy_matcher.clone();
                Task::future(sort_appliations(entry_infos, filter_text, matcher))
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let entries = self
            .sorted_applications
            .clone()
            .into_iter()
            .take(self.config.max_entries)
            .enumerate()
            .map(|(index, scored_entry)| {
                create_entry_card(self.current_focus, (index, scored_entry.entry))
            })
            .collect::<Vec<_>>();
        let entry_container = Column::from_vec(entries)
            .width(Length::Fill)
            .spacing(MEDIUM_SPACING);
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
            .width(Length::Fill)
            .spacing(MEDIUM_SPACING),
        )
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        event::listen_with(move |event, _status, _id| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modifiers: modifier,
                key: iced::keyboard::key::Key::Named(key),
                modified_key: _,
                physical_key: _,
                location: _,
                text: _,
            }) => match key {
                Named::Escape => Some(Message::Exit),
                Named::Enter => Some(Message::LaunchFocusedEntry),
                Named::Tab => match modifier {
                    Modifiers::SHIFT => Some(Message::MoveApplicationFocus(FocusDirection::Up)),
                    _ => Some(Message::MoveApplicationFocus(FocusDirection::Down)),
                },
                _ => None,
            },
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
        SCALE_FACTOR
    }
}
