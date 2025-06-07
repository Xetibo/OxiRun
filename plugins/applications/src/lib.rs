use std::{
    cell::RefCell,
    collections::HashMap,
    env,
    fmt::Debug,
    fs::{self, DirEntry},
    io::BufRead,
    path::PathBuf,
    process::Command,
    sync::Arc,
};

use config::{Config, get_config};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use iced::{
    Alignment, Element, Length, Task,
    widget::{Row, container, text},
};
use oxiced::any_send::OxiAny;
use toml::Table;

mod config;

const SVG_ENDING: &'static str = ".svg";
const PNG_ENDING: &'static str = ".png";

// https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html
const FREEDESKTOP_FIELDS: [&str; 13] = [
    "%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%k", "%v", "%m",
];

const DATA_DIRS: [&str; 2] = ["XDG_DATA_DIRS", "XDG_DATA_HOME"];

const ICON_SIZE: f32 = 60.0;
const SORT_THRESHOLD: i64 = 25;

#[derive(Default)]
pub struct Model {
    config: Config,
    applications: Vec<EntryInfo>,
    sorted_applications: Vec<ScoredEntryInfo>,
    fuzzy_matcher: Arc<SkimMatcherV2>,
}

impl Model {
    pub fn new(global_config: Table) -> Model {
        let config = get_config(global_config);
        Model {
            config,
            ..Default::default()
        }
    }
}

impl Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{:?} {:?}",
            &self.applications, &self.sorted_applications
        ))
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ReceiveEntries(Vec<EntryInfo>),
    ReceiveSortedEntries(Vec<ScoredEntryInfo>),
}

#[derive(Debug, Clone)]
pub enum IconVariant {
    Svg(PathBuf),
    Png(PathBuf),
    Invalid,
}

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub name: String,
    pub icon: Option<IconVariant>,
    pub categories: Vec<String>,
    pub exec: String,
}

#[derive(Debug, Clone)]
pub struct ScoredEntryInfo {
    pub score: i64,
    pub entry: EntryInfo,
}

fn read_single_icon(
    iconmap: &mut HashMap<String, IconVariant>,
    file_res: Result<DirEntry, std::io::Error>,
) {
    match file_res {
        Ok(file) => {
            let filename = file.file_name().into_string().unwrap_or_default();
            let (stripped_name, icon_variant) = if filename.ends_with(PNG_ENDING) {
                (
                    filename.trim_end_matches(PNG_ENDING),
                    IconVariant::Png(file.path()),
                )
            } else if filename.ends_with(SVG_ENDING) {
                (
                    filename.trim_end_matches(SVG_ENDING),
                    IconVariant::Svg(file.path()),
                )
            } else {
                (filename.as_str(), IconVariant::Invalid)
            };
            iconmap.insert(stripped_name.to_string(), icon_variant);
        }
        _ => (),
    };
}

fn read_icons_per_dir(path: String) -> HashMap<String, IconVariant> {
    let mut map = HashMap::new();
    // TODO make this use the current theme from gtk ?
    // perhaps make kde configurable? or get from env?
    for theme in ["hicolor", "Adwaita"] {
        match fs::read_dir(format!("{}/icons/{}", path, theme)) {
            Ok(dirs) => {
                for subdirs in dirs {
                    if let Ok(dir) = subdirs {
                        match fs::read_dir(format!("{}/apps", dir.path().to_str().unwrap_or(""))) {
                            Ok(files) => {
                                for file_res in files {
                                    read_single_icon(&mut map, file_res);
                                }
                            }
                            _ => (),
                        };
                    };
                }
            }
            _ => (),
        }
    }
    match fs::read_dir(format!("{}/pixmaps", path)) {
        Ok(files) => {
            for file_res in files {
                read_single_icon(&mut map, file_res);
            }
        }
        _ => (),
    }
    map
}

fn read_single_entry(
    config: &Config,
    iconmap: &HashMap<String, IconVariant>,
    entries: &mut HashMap<String, EntryInfo>,
    file: DirEntry,
) {
    match fs::read(file.path()) {
        Ok(data) => {
            let mut map = HashMap::new();
            let mut iter = data.lines();
            if iter
                .next()
                .unwrap_or(Ok("".to_string()))
                .unwrap_or("".to_string())
                != "[Desktop Entry]"
            {
                return;
            }
            for line_res in iter {
                if let Ok(line) = line_res {
                    if line.starts_with("[Desktop Action") {
                        break;
                    }
                    if let Some((left, right)) = line.split_once("=") {
                        let key = left.to_string();
                        if map.get(&key).is_none() {
                            map.insert(key, right.to_string());
                        }
                    }
                }
            }

            // NoDisplay is set for applications which should not be shown in a runner
            if let Some("true") = map.get("NoDisplay").map(String::as_str) {
                return;
            }

            let exec = map.get("Exec").map(|val| {
                let mut exec = val.to_string();
                for field in FREEDESKTOP_FIELDS {
                    // TODO should this be possible to be used with additional text
                    // in the text field?
                    exec = exec.replace(field, "");
                }
                if let Some("true") = map.get("Terminal").map(String::as_str) {
                    exec = config.terminal.clone() + " " + &exec;
                }
                exec
            });
            let name = map.get("Name").map(|val| val.to_string());
            let icon = map.get("Icon").map(|val| {
                if let Some(icon) = iconmap.get(val) {
                    icon.clone()
                } else if val.ends_with(PNG_ENDING) {
                    IconVariant::Png(PathBuf::from(val))
                } else if val.ends_with(SVG_ENDING) {
                    IconVariant::Svg(PathBuf::from(val))
                } else {
                    IconVariant::Invalid
                }
            });
            let category_entries = map.get("Categories");
            let keyword_entries = map.get("Keywords");
            let categories = category_entries
                .iter()
                .zip(keyword_entries)
                .map(|(categories, keywords)| {
                    let mut entries = Vec::new();
                    let category_iter = categories.split(";");
                    let keyword_iter = keywords.split(";");
                    for (category, keyword) in category_iter.zip(keyword_iter) {
                        entries.push(category.to_string());
                        entries.push(keyword.to_string());
                    }
                    entries
                })
                .flatten()
                .collect::<Vec<_>>();
            match (name, exec) {
                (None, None) => (),
                (None, Some(_)) => (),
                (Some(_), None) => (),
                (Some(name), Some(exec)) => {
                    entries.insert(name.clone(), EntryInfo {
                        name,
                        icon,
                        categories,
                        exec,
                    });
                }
            }
        }
        Err(_) => (),
    }
}

fn read_entry_of_dirs(
    config: &Config,
    iconmap: &HashMap<String, IconVariant>,
    path: String,
) -> HashMap<String, EntryInfo> {
    let mut entries = HashMap::new();
    match fs::read_dir(format!("{}/applications", path)) {
        Ok(files) => {
            for file_res in files {
                match file_res {
                    Ok(file) => {
                        if file
                            .file_name()
                            .to_str()
                            .unwrap_or_default()
                            .ends_with(".desktop")
                        {
                            read_single_entry(config, iconmap, &mut entries, file);
                        }
                    }
                    Err(_) => (),
                }
            }
        }
        Err(_) => (),
    };
    entries
}

pub fn fetch_entries(config: Config) -> Message {
    let dir_iter = DATA_DIRS
        .into_iter()
        .map(|val| {
            let dirs_res = env::var(val);
            if let Ok(dirs) = dirs_res {
                dirs.split(":").map(String::from).collect::<Vec<String>>()
            } else {
                // TODO handle error
                Vec::new()
            }
        })
        .flatten();

    let iconmap = dir_iter
        .clone()
        .map(read_icons_per_dir)
        .flatten()
        .collect::<HashMap<String, IconVariant>>();

    let entries = dir_iter
        .map(|val| read_entry_of_dirs(&config, &iconmap, val))
        .flatten()
        .collect::<HashMap<String, EntryInfo>>()
        .into_values()
        .collect::<Vec<_>>();

    Message::ReceiveEntries(entries)
}

pub fn create_entry_card<'a>(entry: EntryInfo) -> Element<'a, Message> {
    let icon = entry
        .icon
        .as_ref()
        .map(|icon| match icon {
            IconVariant::Svg(path_buf) => {
                let handle = iced::widget::svg::Handle::from_path(path_buf);
                let widget: Element<Message> = iced::widget::svg(handle)
                    .height(Length::Fixed(ICON_SIZE))
                    .width(Length::Fixed(ICON_SIZE))
                    .into();
                Some(widget)
            }
            IconVariant::Png(path_buf) => Some(
                iced::widget::image(path_buf)
                    .height(Length::Fixed(ICON_SIZE))
                    .width(Length::Fixed(ICON_SIZE))
                    .into(),
            ),
            IconVariant::Invalid => None,
        })
        .flatten();
    let content = Row::new().push_maybe(icon).push(
        container(
            text(entry.name.clone())
                .align_y(Alignment::Center)
                .height(Length::Fill),
        )
        .align_right(Length::Fill),
    );
    content.into()
}

pub fn sort_appliations(
    applications: Vec<EntryInfo>,
    filter_text: String,
    fuzzy_matcher: Arc<SkimMatcherV2>,
) -> Message {
    let mut sorted_applications = applications
        .clone()
        .into_iter()
        .filter_map(|entry| {
            let mut category_scores = Vec::new();
            let name_score = fuzzy_matcher.fuzzy_match(&entry.name, &filter_text);

            for category in entry.categories.iter() {
                category_scores.push(
                    fuzzy_matcher
                        .fuzzy_match(&category, &filter_text)
                        .unwrap_or(0),
                );
            }

            let name_max = name_score.unwrap_or(0);
            let score = *category_scores.iter().max().unwrap_or(&0).max(&name_max);
            if score < SORT_THRESHOLD {
                None
            } else {
                Some(ScoredEntryInfo { score, entry })
            }
        })
        .collect::<Vec<_>>();
    sorted_applications.sort_by(|first, second| second.score.cmp(&first.score));
    Message::ReceiveSortedEntries(sorted_applications)
}

pub fn to_oxiany_rc(msg: Message) -> Arc<dyn OxiAny> {
    let boxed = Box::leak(Box::new(msg));
    Arc::new(boxed as &dyn OxiAny)
}

pub async fn to_oxiany_async(msg: Message) -> Arc<dyn OxiAny> {
    let boxed = Box::leak(Box::new(msg));
    Arc::new(boxed as &dyn OxiAny)
}

pub fn run_command(command: &str) {
    let res = Command::new("sh").arg("-c").arg(command).spawn();
    if let Err(error) = res {
        panic!("Failed to spawn command: {}", error.to_string());
    }
}

#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn model(
    global_config: Table,
) -> (
    Arc<RefCell<&'static mut dyn OxiAny>>,
    Option<Task<Arc<dyn OxiAny>>>,
) {
    let model = Box::leak(Box::new(Model::new(global_config)));

    let config = model.config.clone();
    // TODO get config from main app perhaps? if so how? this should only take subkeys
    (
        Arc::new(RefCell::new(model as &'static mut dyn OxiAny)),
        Some(Task::future(to_oxiany_async(fetch_entries(config)))),
    )
}

#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn update(
    filter_text: String,
    model: Arc<RefCell<&'static mut dyn OxiAny>>,
    msg: Arc<&'static mut dyn OxiAny>,
) -> Option<Task<Arc<dyn OxiAny>>> {
    let mut model_borrow = model.borrow_mut();
    let model = model_borrow
        .downcast_mut::<Model>()
        .expect("can't get model in update");
    let msg = msg
        .downcast_ref::<Message>()
        .expect("can't get msg in update")
        .to_owned();
    match msg {
        Message::ReceiveEntries(entry_infos) => {
            model.applications = entry_infos.clone();
            let matcher = model.fuzzy_matcher.clone();
            Some(Task::future(to_oxiany_async(sort_appliations(
                entry_infos.clone(),
                filter_text,
                matcher,
            ))))
        }
        Message::ReceiveSortedEntries(scored_entry_infos) => {
            model.sorted_applications = scored_entry_infos;
            None
        }
    }
}

#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn sort(
    filter_text: String,
    model: Arc<RefCell<&'static mut dyn OxiAny>>,
) -> Option<Task<Arc<dyn OxiAny>>> {
    let mut model_borrow = model.borrow_mut();
    let model = model_borrow
        .downcast_mut::<Model>()
        .expect("can't get model in sort");
    let applications = model.applications.clone();
    Some(Task::future(to_oxiany_async(sort_appliations(
        applications,
        filter_text,
        model.fuzzy_matcher.clone(),
    ))))
}

#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn launch(
    focused_index: usize,
    model: Arc<RefCell<&'static mut dyn OxiAny>>,
) -> Option<Task<&'static dyn OxiAny>> {
    let mut model_borrow = model.borrow_mut();
    let model = model_borrow
        .downcast_mut::<Model>()
        .expect("can't get model in sort");
    let exec = &model
        .sorted_applications
        .get(focused_index)
        .expect("Could not get entry for index")
        .entry
        .exec;
    run_command(&exec);
    None
}

#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn view(
    model: Arc<RefCell<&'static mut dyn OxiAny>>,
) -> Result<Vec<(i64, Element<'static, Arc<dyn OxiAny>>)>, std::io::Error> {
    let model_borrow = model.borrow();
    let model = model_borrow
        .downcast_ref::<Model>()
        .expect("can't get model in sort");
    let entries: Vec<(i64, Element<Arc<dyn OxiAny>>)> = model
        .sorted_applications
        .clone()
        .into_iter()
        .take(model.config.max_entries)
        .map(|scored_entry| {
            (
                scored_entry.score,
                Into::<Element<Message>>::into(create_entry_card(scored_entry.entry))
                    .map(to_oxiany_rc),
            )
        })
        .collect::<Vec<_>>();
    Ok(entries)
}

#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn count(model: Arc<RefCell<&'static mut dyn OxiAny>>) -> usize {
    let model_borrow = model.borrow();
    let model = model_borrow
        .downcast_ref::<Model>()
        .expect("can't get model in sort");
    usize::min(model.sorted_applications.len(), model.config.max_entries)
}
