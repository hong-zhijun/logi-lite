pub mod platform;
pub mod shared;

// Preserve existing imports while separating OS adapters from shared logic.
#[cfg(windows)]
pub use platform::windows::{devices, input};
pub use shared::{config, hidpp};
