use crate::error::internal_error::InternalError;
use crate::repository::handler::Repository;
use crate::repository::settings::RepositoryConfig;
use crate::storage::models::Storage;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLockReadGuard;

pub struct CIHandler<StorageType: Storage> {
    config: RepositoryConfig,
    storage: Arc<StorageType>,
}
impl<StorageType: Storage> CIHandler<StorageType> {
    pub async fn create(
        config: RepositoryConfig,
        storage: Arc<StorageType>,
    ) -> Result<CIHandler<StorageType>, InternalError> {
        Ok(CIHandler { config, storage })
    }
}

#[async_trait]
impl<StorageType: Storage> Repository<StorageType> for CIHandler<StorageType> {
    fn get_repository(&self) -> &RepositoryConfig {
        &self.config
    }

    fn get_storage(&self) -> &StorageType {
        &self.storage
    }
}
