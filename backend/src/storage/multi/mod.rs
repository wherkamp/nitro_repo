use lockfree::map::Map;

use std::mem;
use std::path::PathBuf;
use std::sync::Arc;

use crate::repository::handler::DynamicRepositoryHandler;
use log::{error, info};
use serde::Deserialize;

use tokio::fs::{read_to_string, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::storage::bad_storage::BadStorage;
use crate::storage::error::StorageError;
use crate::storage::file::StorageFile;

use crate::storage::error::StorageError::StorageCreateError;
use crate::storage::models::Storage;
use crate::storage::{DynamicStorage, StorageSaver};

pub mod web;

async fn load_storages(
    storages_file: &PathBuf,
) -> Result<Map<String, Arc<DynamicStorage>>, StorageError> {
    if !storages_file.exists() {
        return Ok(Map::new());
    }
    let string = read_to_string(&storages_file).await?;
    let result: Vec<StorageSaver> = serde_json::from_str(&string)?;
    let values: Map<String, Arc<DynamicStorage>> = Map::new();
    for factory in result {
        let name = factory.generic_config.id.clone();
        let storage = match DynamicStorage::new(factory).await {
            Ok(value) => value,
            Err((error, factory)) => DynamicStorage::BadStorage(BadStorage::create(factory, error)),
        };
        values.insert(name, Arc::new(storage));
    }
    Ok(values)
}

pub async fn save_storages(
    storages: Vec<StorageSaver>,
    storages_file: &PathBuf,
) -> Result<(), StorageError> {
    let result = serde_json::to_string(&storages)?;
    let mut file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(storages_file)
        .await?;
    file.write_all(result.as_bytes()).await?;
    Ok(())
}
#[derive(Debug)]
pub struct MultiStorageController<S: Storage> {
    pub storages: Map<String, Arc<S>>,
    pub unloaded_storages: Map<String, Arc<S>>,
    pub storage_file: PathBuf,
}
#[derive(Debug, Deserialize)]
pub enum PurgeLevel {
    All,
    Configs,
    RemoveFromList,
}
impl MultiStorageController<DynamicStorage> {
    pub async fn init(
        storages_file: PathBuf,
    ) -> Result<MultiStorageController<DynamicStorage>, StorageError> {
        info!("Loading Storages");
        let result = load_storages(&storages_file).await?;
        let mut controller = MultiStorageController {
            storages: Map::new(),
            unloaded_storages: result,
            storage_file: storages_file,
        };
        controller.load_unloaded_storages().await?;
        Ok(controller)
    }
    pub async fn get_storage_by_name(
        &self,
        name: &str,
    ) -> Result<Option<Arc<DynamicStorage>>, StorageError> {
        let storages = self.storages.get(name);
        if let Some(storage) = storages {
            Ok(Some(storage.as_ref().1.clone()))
        } else {
            Ok(None)
        }
    }
    pub fn does_storage_exist(&self, name: &str) -> Result<bool, StorageError> {
        let storages = self.storages.get(name);
        Ok(storages.is_some())
    }

    /// Attempts to run the storages load on any storages that are unloaded.
    /// This will include the Error storages
    pub async fn load_unloaded_storages<'a>(&mut self) -> Result<(), StorageError> {
        let unloaded = mem::take(&mut self.unloaded_storages);
        for (name, storage) in unloaded.into_iter() {
            match storage.get_repos_to_load().await {
                Ok(repositories) => {
                    for (name, repository) in repositories.into_iter() {
                        info!("Loading repository {}", name);
                        let handler =
                            DynamicRepositoryHandler::new_dyn_storage(storage.clone(), repository)
                                .await
                                .map_err(|error| {
                                    error!("Error loading repository {}. Error {}", name, error);
                                });
                        if let Ok(handler) = handler {
                            storage.add_repo_loaded(handler)?;
                        }
                    }
                }
                Err(error) => {
                    error!("Error loading storages {}: {}", name, error);
                }
            }
            self.storages.insert(name, storage);
        }
        Ok(())
    }

    /// Starts by checking all the storages to see if the name already exists
    /// Collects all the StorageSavers into an Array.
    /// Checks to ensure the storages will load correctly. If it will not it will error our
    /// Saves the new storages config
    /// Adds the storages to the main Storage map without loading repositories. Because its a new storages
    pub async fn create_storage<'a>(&self, storage: StorageSaver) -> Result<(), StorageError> {
        let name = storage.generic_config.id.clone();
        // Check if the storages already exists then collect all Vec<StorageSaver> and add the new one
        let mut storages = Vec::new();
        for storages_file in self.storages.iter() {
            if storages_file.key().eq(&name) {
                return Err(StorageCreateError("Storage already exists".to_string()));
            }

            storages.push(storages_file.val().storage_config().clone());
        }
        let storage = DynamicStorage::create_new(storage)
            .await
            .map_err(|(error, v)| {
                error!("Error creating storages {:?}.", v);
                StorageCreateError(error.to_string())
            })?;
        storages.push(storage.storage_config().clone());
        save_storages(storages, &self.storage_file).await?;

        self.storages.insert(name, Arc::new(storage));
        Ok(())
    }

    /// Follows the same steps as create_storage but will treat the new storages as something that has data in it.
    pub async fn recover_storage(&self, storage: StorageSaver) -> Result<(), StorageError> {
        let name = storage.generic_config.id.clone();
        // Check if the storages already exists then collect all Vec<StorageSaver> and add the new one
        let mut storages = Vec::new();
        for storages_file in self.storages.iter() {
            if storages_file.key().eq(&name) {
                return Err(StorageCreateError("Storage already exists".to_string()));
            }

            storages.push(storages_file.val().storage_config().clone());
        }
        let storage = DynamicStorage::create_new(storage)
            .await
            .map_err(|(error, v)| {
                error!("Error creating storages {:?}.", v);
                StorageCreateError(error.to_string())
            })
            .map(Arc::new)?;
        let repositories = storage.get_repos_to_load().await?;
        for (name, repository) in repositories.into_iter() {
            info!("Loading repository {} From Recovery", name);
            let handler = DynamicRepositoryHandler::new_dyn_storage(storage.clone(), repository)
                .await
                .map_err(|error| {
                    error!("Error loading repository {}. Error {}", name, error);
                });
            if let Ok(handler) = handler {
                storage.add_repo_loaded(handler)?;
            }
        }

        storages.push(storage.storage_config().clone());
        save_storages(storages, &self.storage_file).await?;

        self.storages.insert(name, storage);
        Ok(())
    }

    pub async fn delete_storage(
        &self,
        storage: impl AsRef<str>,
        purge_level: PurgeLevel,
    ) -> Result<(), StorageError> {
        let option = self.storages.remove(storage.as_ref()).ok_or_else(|| {
            StorageError::StorageDeleteError("Storage does not exist".to_string())
        })?;
        save_storages(self.storage_savers().await, &self.storage_file).await?;

        match purge_level {
            PurgeLevel::All => {
                let x = option.val().get_repository_list()?;
                for repository in x.into_iter() {
                    if let Err(error) = option.val().delete_repository(&repository.name, true).await
                    {
                        error!(
                            "Error deleting repository {}. Error {}",
                            repository.name, error
                        );
                    }
                }
            }
            PurgeLevel::Configs => {
                let x = option.val().get_repository_list()?;
                for repository in x.into_iter() {
                    if let Err(error) = option
                        .val()
                        .delete_repository(&repository.name, false)
                        .await
                    {
                        error!(
                            "Error deleting repository {}. Error {}",
                            repository.name, error
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn storage_savers(&self) -> Vec<StorageSaver> {
        let mut result = Vec::new();
        for x in self.storages.iter() {
            let saver = x.val().storage_config().clone();
            result.push(saver);
        }
        result
    }
    pub async fn names(&self) -> Vec<String> {
        self.storages
            .iter()
            .map(|v| v.key().clone())
            .collect::<Vec<_>>()
    }
    pub async fn storages_as_file_list(&self) -> Result<Vec<StorageFile>, StorageError> {
        let mut files = Vec::new();
        for v in self.storages.iter() {
            let name = v.0.clone();
            let create = v.1.as_ref().storage_config().generic_config.created;
            files.push(StorageFile {
                name: name.clone(),
                full_path: name.clone(),
                mime: "text/directory".to_string(),
                directory: true,
                file_size: 0,
                modified: None,
                created: create,
            });
        }
        Ok(files)
    }
}
