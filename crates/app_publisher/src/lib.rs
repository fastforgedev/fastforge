mod appgallery;
mod appstore;
mod common;
mod custom;
mod fir;
mod firebase;
mod firebase_hosting;
mod github;
mod pgyer;
mod playstore;
mod repository;
mod s3;
mod vercel;

pub use fastforge_core::{
    AppPublisher, PublishConfig, PublishError, PublishProgressCallback, PublishResult,
};

pub use appgallery::AppGalleryPublisher;
pub use appstore::AppStorePublisher;
pub use custom::CustomPublisher;
pub use fir::FirPublisher;
pub use firebase::FirebasePublisher;
pub use firebase_hosting::FirebaseHostingPublisher;
pub use github::GitHubPublisher;
pub use pgyer::PgyerPublisher;
pub use playstore::PlayStorePublisher;
pub use repository::{AptRepoPublisher, PacmanRepoPublisher};
pub use s3::{CosPublisher, MinioPublisher, OssPublisher, QiniuPublisher, S3Publisher};
pub use vercel::VercelPublisher;
