use iced::{Element, Task};
use libloading::Library;
use oxiced::any_send::OxiAny;

use crate::PluginFuncs;

pub fn load_plugin(lib: &'static Library) -> Option<PluginFuncs> {
    unsafe {
        let model: Result<
            libloading::Symbol<
                unsafe extern "C" fn() -> (
                    &'static mut dyn OxiAny,
                    Option<Task<&'static mut dyn OxiAny>>,
                ),
            >,
            libloading::Error,
        > = lib.get(b"model");
        let update: Result<
            libloading::Symbol<
                unsafe extern "C" fn(
                    data: &&mut dyn OxiAny,
                    msg: &dyn OxiAny,
                ) -> Option<Task<&'static dyn OxiAny>>,
            >,
            libloading::Error,
        > = lib.get(b"update");
        let view: Result<
            libloading::Symbol<
                unsafe extern "C" fn(
                    data: &dyn OxiAny,
                )
                    -> Result<Element<&'static mut dyn OxiAny>, std::io::Error>,
            >,
            libloading::Error,
        > = lib.get(b"view");

        match (model, update, view) {
            (Ok(model), Ok(update), Ok(view)) => Some(PluginFuncs {
                model,
                update,
                view,
            }),
            _ => None,
        }
    }
}
