use crate::app::ui::views::error_view::ErrorView;
use crate::app::ui::views::playing_view::PlayingView;
use crate::app::ui::views::rom_select_view::RomSelectView;
use crate::app::ui::views::waiting_view::WaitingView;

pub mod error_view;
pub mod playing_view;
pub mod rom_select_view;
pub mod waiting_view;

pub enum UiView {
    Waiting(WaitingView),
    RomSelect(RomSelectView),
    Options,
    Playing(PlayingView),
    Error(ErrorView),
}

impl UiView {
    pub(crate) fn playing() -> Self {
        UiView::Playing(PlayingView::new())
    }
}
