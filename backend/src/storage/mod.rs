pub mod admin;
pub mod bad_storage;
pub mod error;
pub mod local_storage;
pub mod models;
pub mod multi;
pub mod file;

pub static STORAGES_CONFIG: &str = "storages.nitro_repo";
pub static STORAGE_CONFIG: &str = "storage.nitro_repo";
