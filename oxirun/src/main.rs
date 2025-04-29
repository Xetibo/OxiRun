use std::collections::HashMap;
use std::sync::Arc;

use applications::{
    EntryInfo, ScoredEntryInfo, create_entry_card, fetch_entries, run_command, sort_appliations,
};
use config::{Config, get_config, get_oxirun_dir};
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::{Column, column};
use iced::{Element, Length, Subscription, Task, Theme, event};
use oxiced::any_send::OxiAny;
use oxiced::theme::get_theme;
use oxiced::widgets::oxi_text_input::text_input;

use iced_layershell::Application;
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use plugins::load_plugin;
use utils::{FocusDirection, MEDIUM_SPACING, wrap_in_rounded_box};

mod applications;
mod config;
mod plugins;
mod utils;

// TODO make this configurable
const ICON_SIZE: f32 = 50.0;
const SORT_THRESHOLD: i64 = 25;
const SCALE_FACTOR: f64 = 1.0;
const WINDOW_SIZE: (u32, u32) = (600, 600);
const WINDOW_MARGINS: (i32, i32, i32, i32) = (100, 100, 100, 100);
const WINDOW_LAYER: Layer = Layer::Overlay;
const WINDOW_KEYBAORD_MODE: KeyboardInteractivity = KeyboardInteractivity::Exclusive;

#[derive(Clone, Debug)]
pub struct PluginFuncs {
    pub model: libloading::Symbol<
        'static,
        unsafe extern "C" fn() -> (
            &'static mut dyn OxiAny,
            Option<Task<&'static mut dyn OxiAny>>,
        ),
    >,
    pub update: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            data: &&mut dyn OxiAny,
            msg: &dyn OxiAny,
        ) -> Option<Task<&'static dyn OxiAny>>,
    >,
    pub view: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            data: &dyn OxiAny,
        ) -> Result<Element<&'static mut dyn OxiAny>, std::io::Error>,
    >,
}

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
    plugins: HashMap<usize, (&'static mut dyn OxiAny, PluginFuncs)>,
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
            plugins: HashMap::new(),
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
    PluginMsg(usize, Arc<&'static mut dyn OxiAny>),
    ErrorMsg, // TODO use
}

impl TryInto<LayershellCustomActions> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayershellCustomActions, Self::Error> {
        Err(self)
    }
}

fn get_plugins() -> (
    HashMap<usize, (&'static mut dyn OxiAny, PluginFuncs)>,
    Vec<Task<Message>>,
) {
    let mut plugins = HashMap::new();
    let mut tasks = Vec::new();
    // TODO make configurable
    let plugin_dir = get_oxirun_dir().join("plugins");
    if !plugin_dir.is_dir() {
        std::fs::create_dir(&plugin_dir).expect("Could not create config dir");
    }
    for (index, res) in plugin_dir
        .read_dir()
        .expect("Could not read plugin directory")
        .enumerate()
    {
        match res {
            Ok(file) => {
                if file
                    .file_name()
                    .to_str()
                    .unwrap_or_default()
                    .ends_with(".so")
                {
                    unsafe {
                        let lib = Box::leak(Box::new(
                            libloading::Library::new(&file.path()).expect("Could not load library"),
                        ));
                        if let Some(plugin) = load_plugin(lib) {
                            let (model, task_opt) = (plugin.model.clone())();
                            plugins.insert(index, (model, plugin));
                            if let Some(task) = task_opt.map(|val| {
                                val.map(|inner| {
                                    inner
                                        .downcast_ref::<Message>()
                                        .unwrap_or(&Message::ErrorMsg)
                                        .clone()
                                })
                            }) {
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

impl Application for OxiRun {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Task<Message>) {
        let config = get_config();
        let (plugins, mut plugin_tasks) = get_plugins();
        plugin_tasks.push(iced::widget::text_input::focus("search_box"));
        plugin_tasks.push(Task::future(fetch_entries(config.clone())));
        (
            Self {
                config,
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
            Message::PluginMsg(index, msg) => unsafe {
                let plugin = self.plugins.get(&index).unwrap();
                let update_func = plugin.1.update.clone();
                let task_opt = (update_func)(&plugin.0, &msg);
                if let Some(task) = task_opt {
                    task.map(|val| {
                        val.downcast_ref::<Message>()
                            .expect("Could not get follow up task from plugin")
                            .clone()
                    })
                } else {
                    Task::none()
                }
            },
            Message::ErrorMsg => {
                println!("error occurred");
                Task::none()
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
        //let entry_container = Column::from_vec(entries)
        //    .width(Length::Fill)
        //    .spacing(MEDIUM_SPACING);

        let plugin_views = self.plugins.iter().map(|(index, (model, funcs))| {
            let view_func = funcs.view.clone();
            let view_res = unsafe { (view_func)(model) };
            match view_res {
                Ok(view) => Ok(view.map(move |msg| Message::PluginMsg(*index, Arc::new(msg)))),
                Err(err) => Err(err),
            }
        });
        let mut col = Column::new();
        col = col.push(
            text_input(
                "Enter text to find",
                self.filter_text.as_str(),
                Message::SetFilterText,
            )
            .id("search_box"),
        );
        for plugin_view in plugin_views {
            // TODO handle error
            col = col.push_maybe(plugin_view.ok());
        }
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
