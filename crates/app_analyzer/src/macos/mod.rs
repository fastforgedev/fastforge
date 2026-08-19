pub mod app;
pub mod bundle;
pub mod dmg;

mod asar;
mod components;
mod fs_stats;
mod signature;
mod techstack;

pub use app::MacOSAppAnalyzer;
pub use dmg::MacOSDmgAnalyzer;
