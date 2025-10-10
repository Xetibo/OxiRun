use std::sync::{Arc, RwLock};

use iced::{Element, Task};
use libloading::Library;
use oxiced::any_send::OxiAny;
use toml::Table;

pub type PluginModel = Arc<RwLock<&'static mut dyn OxiAny>>;
pub type PluginMsg = Arc<&'static mut dyn OxiAny>;

#[allow(improper_ctypes_definitions)]
#[derive(Clone, Debug)]
pub struct PluginFuncs {
    pub model: libloading::Symbol<
        'static,
        unsafe extern "C" fn(Table) -> (PluginModel, Option<Task<PluginMsg>>),
    >,
    pub update: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            filter_text: String,
            model: PluginModel,
            msg: PluginMsg,
        ) -> Option<Task<PluginMsg>>,
    >,
    pub sort: libloading::Symbol<
        'static,
        unsafe extern "C" fn(filter_text: String, model: PluginModel) -> Option<Task<PluginMsg>>,
    >,
    pub launch: libloading::Symbol<
        'static,
        unsafe extern "C" fn(focused_index: usize, model: PluginModel) -> Option<Task<PluginMsg>>,
    >,
    /// The i64 represents the score of each element, this can also be used to ensure your plugin is at
    /// the top or close to the top
    pub view: libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            model: PluginModel,
        )
            -> Result<Vec<(i64, Element<'static, PluginMsg>)>, std::io::Error>,
    >,
    pub errors:
        libloading::Symbol<'static, unsafe extern "C" fn(model: PluginModel) -> Vec<String>>,
    pub name: libloading::Symbol<'static, unsafe extern "C" fn() -> &'static str>,
    pub count: libloading::Symbol<'static, unsafe extern "C" fn(model: PluginModel) -> usize>,
}

pub fn load_plugin(lib: &'static Library) -> Option<PluginFuncs> {
    unsafe {
        let model: Result<
            libloading::Symbol<
                unsafe extern "C" fn(Table) -> (PluginModel, Option<Task<PluginMsg>>),
            >,
            libloading::Error,
        > = lib.get(b"model");
        let update: Result<
            libloading::Symbol<
                unsafe extern "C" fn(
                    filter_text: String,
                    model: Arc<RwLock<&mut dyn OxiAny>>,
                    msg: PluginMsg,
                ) -> Option<Task<PluginMsg>>,
            >,
            libloading::Error,
        > = lib.get(b"update");
        let sort: Result<
            libloading::Symbol<
                unsafe extern "C" fn(
                    filter_text: String,
                    model: PluginModel,
                ) -> Option<Task<PluginMsg>>,
            >,
            libloading::Error,
        > = lib.get(b"sort");
        let launch: Result<
            libloading::Symbol<
                unsafe extern "C" fn(
                    focused_index: usize,
                    model: PluginModel,
                ) -> Option<Task<PluginMsg>>,
            >,
            libloading::Error,
        > = lib.get(b"launch");
        let view: Result<
            libloading::Symbol<
                unsafe extern "C" fn(
                    model: PluginModel,
                ) -> Result<
                    Vec<(i64, Element<'static, PluginMsg>)>,
                    std::io::Error,
                >,
            >,
            libloading::Error,
        > = lib.get(b"view");
        let errors: Result<
            libloading::Symbol<unsafe extern "C" fn(model: PluginModel) -> Vec<String>>,
            libloading::Error,
        > = lib.get(b"errors");
        let name: Result<
            libloading::Symbol<unsafe extern "C" fn() -> &'static str>,
            libloading::Error,
        > = lib.get(b"name");
        let count: Result<
            libloading::Symbol<unsafe extern "C" fn(model: PluginModel) -> usize>,
            libloading::Error,
        > = lib.get(b"count");

        match (model, update, sort, launch, view, errors, name, count) {
            (
                Ok(model),
                Ok(update),
                Ok(sort),
                Ok(launch),
                Ok(view),
                Ok(errors),
                Ok(name),
                Ok(count),
            ) => Some(PluginFuncs {
                model,
                update,
                view,
                sort,
                launch,
                errors,
                name,
                count,
            }),
            _ => None,
        }
    }
}
