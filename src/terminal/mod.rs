/// Terminal module - UI and input handling

pub mod input;
pub mod renderer;

pub use input::{InputAction, InputEditor};
pub use renderer::TerminalRenderer;
