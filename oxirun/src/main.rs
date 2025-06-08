use std::collections::HashMap;

use config::{get_allowed_plugins, get_config, get_oxirun_dir};
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::{Column, Row, button, text};
use iced::{Element, Length, Subscription, Task, Theme, event};
use oxiced::theme::get_theme;
use oxiced::widgets::common::{darken_color, lighten_color};
use oxiced::widgets::oxi_button::{self, ButtonVariant};
use oxiced::widgets::oxi_text_input::text_input;

use iced_layershell::Application;
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use plugins::{PluginFuncs, PluginModel, PluginMsg, load_plugin};
use toml::Table;
use utils::{FocusDirection, MEDIUM_SPACING, wrap_in_rounded_box};

mod config;
mod plugins;
mod utils;

// TODO make this configurable
const ICON_SIZE: f32 = 50.0;
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
    plugins: HashMap<usize, (PluginModel, PluginFuncs)>,
    current_focus: usize,
    _config: Table, // TODO use
}

impl Default for OxiRun {
    fn default() -> Self {
        Self {
            theme: get_theme(),
            filter_text: "".into(),
            plugins: HashMap::new(),
            current_focus: 0,
            _config: Table::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    SetFilterText(String),
    Exit,
    LaunchEntry(usize),
    LaunchFocusedEntry,
    MoveApplicationFocus(FocusDirection),
    PluginSubMsg(usize, PluginMsg),
}

impl TryInto<LayershellCustomActions> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayershellCustomActions, Self::Error> {
        Err(self)
    }
}

fn get_plugins(
    config: &Table,
) -> (
    HashMap<usize, (PluginModel, PluginFuncs)>,
    Vec<Task<Message>>,
) {
    let mut plugins = HashMap::new();
    let mut tasks = Vec::new();
    // TODO make configurable
    let plugin_dir = get_oxirun_dir().join("plugins");
    if !plugin_dir.is_dir() {
        std::fs::create_dir(&plugin_dir).expect("Could not create config dir");
    }
    let allowed_files = get_allowed_plugins(&config);
    for (index, res) in plugin_dir
        .read_dir()
        .expect("Could not read plugin directory")
        .enumerate()
    {
        match res {
            Ok(file) => {
                if allowed_files.contains(&file.file_name().to_str().unwrap_or("")) {
                    unsafe {
                        let lib = Box::leak(Box::new(
                            libloading::Library::new(&file.path()).expect("Could not load library"),
                        ));
                        if let Some(plugin) = load_plugin(lib) {
                            let (model, task_opt) = (plugin.model.clone())(config.clone());
                            plugins.insert(index, (model, plugin));
                            if let Some(task) = task_opt
                                .map(|val| val.map(move |msg| Message::PluginSubMsg(index, msg)))
                            {
                                tasks.push(task)
                            }
                        }
                    }
                }
            }
            Err(_) => (),
        }
    }
    (plugins, tasks)
}

fn content_button(
    focused_index: usize,
    current_index: usize,
    content: Element<Message>,
) -> Element<Message> {
    oxi_button::button(content, ButtonVariant::Primary)
        .on_press(Message::LaunchEntry(current_index))
        .style(move |theme, status| {
            let is_focused = current_index == focused_index;
            let palette = theme.extended_palette().primary;
            let default_style = oxi_button::primary_button(theme, status);
            let background = if status == button::Status::Hovered {
                Some(iced::Background::Color(darken_color(palette.base.color)))
            } else if is_focused {
                default_style.background
            } else {
                Some(iced::Background::Color(lighten_color(palette.base.color)))
            };
            iced::widget::button::Style {
                background,
                ..default_style
            }
        })
        .padding(5.0)
        .width(Length::Fill)
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

fn plugin_launch(model: &mut OxiRun, focused_index: usize) -> Vec<Task<Message>> {
    model
        .plugins
        .iter_mut()
        .filter_map(|(index, (plugin_model, funcs))| {
            let index = *index;
            let launch_func = funcs.launch.clone();
            let task_opt = unsafe { (launch_func)(focused_index, plugin_model.clone()) };
            task_opt.map(move |task| task.map(move |msg| Message::PluginSubMsg(index, msg)))
        })
        .collect::<Vec<_>>()
}

fn plugin_sort(model: &mut OxiRun, filter_text: String) -> Vec<Task<Message>> {
    model
        .plugins
        .iter_mut()
        .filter_map(|(index, (plugin_model, funcs))| {
            let index = *index;
            let sort_func = funcs.sort.clone();
            let task_opt = unsafe { (sort_func)(filter_text.clone(), plugin_model.clone()) };
            task_opt.map(move |task| task.map(move |msg| Message::PluginSubMsg(index, msg)))
        })
        .collect::<Vec<_>>()
}

fn plugin_count(model: &mut OxiRun) -> usize {
    model
        .plugins
        .iter_mut()
        .map(|(_, (model, funcs))| {
            let count_func = funcs.count.clone();
            unsafe { (count_func)(model.clone()) }
        })
        .sum::<usize>()
}

fn error_view<'a>(plugin_name: &'static str, errors: Vec<String>) -> Option<Element<'a, Message>> {
    let mut col = Column::new();
    if errors.is_empty() {
        return None;
    }
    col = col.push(text(plugin_name));
    let error_views = errors
        .into_iter()
        .map(|value| text(value))
        .collect::<Vec<_>>();
    for error in error_views {
        col = col.push(error);
    }
    Some(col.into())
}

impl Application for OxiRun {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let config = get_config();
        let (plugins, mut plugin_tasks) = get_plugins(&config);
        plugin_tasks.push(iced::widget::text_input::focus("search_box"));
        (
            Self {
                _config: config,
                plugins,
                ..Default::default()
            },
            Task::batch(plugin_tasks),
        )
    }

    fn namespace(&self) -> String {
        String::from("OxiRun")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetFilterText(value) => {
                self.filter_text = value.clone();
                Task::batch(plugin_sort(self, value))
            }
            Message::Exit => std::process::exit(0),
            Message::LaunchEntry(focused_index) => {
                let tasks = plugin_launch(self, focused_index);
                Task::batch(tasks).chain(Task::done(Message::Exit))
            }
            Message::MoveApplicationFocus(direction) => {
                self.current_focus = direction.add(self.current_focus, plugin_count(self));
                Task::none()
            }
            Message::LaunchFocusedEntry => {
                let tasks = plugin_launch(self, self.current_focus);
                Task::batch(tasks).chain(Task::done(Message::Exit))
            }
            Message::PluginSubMsg(index, msg) => unsafe {
                let plugin = self.plugins.get_mut(&index).unwrap();
                let update_func = plugin.1.update.clone();
                let task_opt = (update_func)(self.filter_text.clone(), plugin.0.clone(), msg);
                if let Some(task) = task_opt {
                    task.map(move |msg| Message::PluginSubMsg(index, msg))
                } else {
                    Task::none()
                }
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let plugin_views = self
            .plugins
            .iter()
            .map(|(index, (model, funcs))| {
                let view_func = funcs.view.clone();
                let view_res = unsafe { (view_func)(model.clone()) };
                match view_res {
                    Ok(view) => {
                        let mut combined = view
                            .into_iter()
                            .map(move |(score, element)| {
                                (
                                    score,
                                    element.map(|msg| Message::PluginSubMsg(*index, msg.clone())),
                                )
                            })
                            .collect::<Vec<_>>();
                        // TODO do we NEED to sort again? it shouldn't sort more than a bit more
                        // than max entries -> all matching plugin entries but still annoying
                        combined.sort_by(|first, second| second.0.cmp(&first.0));
                        combined.into_iter().map(|val| val.1).collect::<Vec<_>>()
                    }
                    // TODO use error
                    Err(_) => Vec::new(),
                }
            })
            .flatten()
            .enumerate()
            .map(|(elem_index, val)| content_button(self.current_focus, elem_index, val))
            .collect::<Vec<_>>();

        let mut col = Column::new();
        col = col.push(
            text_input(
                "Enter text to find",
                self.filter_text.as_str(),
                Message::SetFilterText,
            )
            .id("search_box"),
        );
        for entry in plugin_views {
            col = col.push(entry);
        }

        let mut plugin_error_views = Row::new();
        for (_, plugin) in self.plugins.iter() {
            unsafe {
                plugin_error_views = plugin_error_views.push_maybe(error_view(
                    (plugin.1.name)(),
                    (plugin.1.errors)(plugin.0.clone()).clone(),
                ))
            }
        }
        col = col.push(plugin_error_views);

        wrap_in_rounded_box(col.width(Length::Fill).spacing(MEDIUM_SPACING))
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
