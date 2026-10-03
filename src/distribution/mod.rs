mod install;
mod uninstall;
mod update;
pub use install::{InstallManifest, InstallReport, install, read_manifest};
pub use uninstall::{purge_user_data, uninstall};
pub use update::{
    UpdateReport, artifact_name, newer_version, update, update_from, update_with_timeout,
    verify_checksum,
};
