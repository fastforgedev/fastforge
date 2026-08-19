pub mod android;
pub mod ios;
pub mod macos;

mod archive;
mod checksum;
mod command;
mod json_util;
mod linking;
mod macho;
mod plist_util;
mod provisioning;
mod sdks;

pub use android::{AndroidAabAnalyzer, AndroidApkAnalyzer};
pub use fastforge_core::{AnalyzeConfig, AnalyzeError, AnalyzeResult, AppAnalyzer};
pub use ios::IOSIpaAnalyzer;
pub use macos::{MacOSAppAnalyzer, MacOSDmgAnalyzer};
