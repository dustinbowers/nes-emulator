use crate::app::ui::error::ErrorInfo;
use crate::app::ui::views::playing_view::PlayingView;
use crate::app::ui::views::waiting_view::WaitingView;

pub mod playing_view;
pub mod error_view;
pub mod waiting_view;

pub enum UiView {
    Waiting(WaitingView),
    Options,
    Playing(PlayingView),
    Error(ErrorInfo),
}

impl UiView {
    pub(crate) fn playing() -> Self {
        UiView::Playing(PlayingView::new())
    }

    pub(crate) fn error_load_rom(err: anyhow::Error) -> Self {
        UiView::Error(ErrorInfo::from_anyhow("Failed to load ROM", err))
    }

    pub(crate) fn error_shared_frame() -> Self {
        UiView::Error(ErrorInfo::new("SharedFrameBuffer not found.", ""))
    }
}
