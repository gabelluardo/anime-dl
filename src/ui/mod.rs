mod input;
mod progress;
mod selector;
mod table;
mod tui;

pub use tui::Tui;

#[derive(thiserror::Error, Debug)]
pub enum TuiError {
    #[error("invalid input")]
    InvalidInput,
}
