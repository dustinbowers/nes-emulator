use crate::app::ui::error::ErrorInfo;
use crate::app::ui::views::error_view::ErrorView;
use crate::app::ui::views::playing_view::PlayingView;
use crate::app::ui::views::waiting_view::WaitingView;

pub mod error_view;
pub mod playing_view;
pub mod waiting_view;

pub enum UiView {
    Waiting(WaitingView),
    Options,
    Playing(PlayingView),
    Error(ErrorView),
}

impl UiView {
    pub(crate) fn playing() -> Self {
        UiView::Playing(PlayingView::new())
    }

    pub(crate) fn error_load_rom(err: anyhow::Error) -> Self {
        let info = ErrorInfo::from_anyhow("Failed to load ROM", err);
        UiView::Error(ErrorView::new(info))
    }

    pub(crate) fn error_shared_frame() -> Self {
        let info = ErrorInfo::new("SharedFrameBuffer not found.", "");
        UiView::Error(ErrorView::new(info))
    }
}
