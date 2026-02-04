use crate::app::ui::views::UiView;

pub mod app_input;
pub mod error;
pub mod file_drop_overlay;
pub mod main_view;
pub mod views;
pub mod waiting_view;

pub enum Transition {
    None,
    Switch(UiView),
    Quit,
}

impl Transition {
    pub(crate) fn to(view: UiView) -> Self {
        Transition::Switch(view)
    }
}
