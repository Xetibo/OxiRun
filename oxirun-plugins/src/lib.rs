use iced::{Element, Task};
use oxiced::any_send::OxiAny;

pub trait OxiRunPlugin {
    unsafe extern "C" fn model(&self, additional_data: &mut dyn OxiAny) -> &'static mut dyn OxiAny;
    unsafe extern "C" fn update(&mut self, msg: &dyn OxiAny) -> Option<Task<&'static dyn OxiAny>>;
    unsafe extern "C" fn view(
        &self,
        data: &dyn OxiAny,
    ) -> Result<Element<&'static mut dyn OxiAny>, std::io::Error>;
}
