pub mod android;
pub mod custom;
pub mod ios;
pub mod linux;
pub mod macos;
pub mod ohos;
pub mod web;
pub mod windows;

pub use fastforge_core::{AppPackager, PackageConfig, PackageError, PackageResult};

// Re-export all packagers for convenient access.
pub use android::aab::AndroidAabPackager;
pub use android::apk::AndroidApkPackager;
pub use custom::CustomPackager;
pub use ios::ipa::IOSIpaPackager;
pub use linux::appimage::LinuxAppImagePackager;
pub use linux::deb::LinuxDebPackager;
pub use linux::direct::LinuxDirectPackager;
pub use linux::pacman::LinuxPacmanPackager;
pub use linux::rpm::LinuxRpmPackager;
pub use linux::zip::LinuxZipPackager;
pub use macos::dmg::MacOSDmgPackager;
pub use macos::pkg::MacOSPkgPackager;
pub use macos::zip::MacOSZipPackager;
pub use ohos::app::OHOSAppPackager;
pub use ohos::hap::OHOSHapPackager;
pub use web::direct::WebDirectPackager;
pub use web::zip::WebZipPackager;
pub use windows::direct::WindowsDirectPackager;
pub use windows::exe::WindowsExePackager;
pub use windows::msix::WindowsMsixPackager;
pub use windows::zip::WindowsZipPackager;
