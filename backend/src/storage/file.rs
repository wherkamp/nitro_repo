use std::fs::{DirEntry, Metadata};
use std::path::PathBuf;
use std::time::SystemTime;

use actix_files::NamedFile;
use actix_web::body::BoxBody;
use actix_web::error::ErrorBadRequest;
use actix_web::http::header::ACCEPT;
use actix_web::http::{Method, StatusCode};
use actix_web::{HttpRequest, HttpResponse, Responder};
use log::{as_error, error, trace};
use serde::Serialize;
use tokio::fs::metadata;

use crate::storage::error::StorageError;

///Storage Files are just a data container holding the file name, directory relative to the root of nitro_repo and if its a directory
#[derive(Serialize, Clone, Debug)]
pub struct StorageFile {
    pub name: String,
    pub full_path: String,
    pub mime: String,
    pub directory: bool,
    pub file_size: u64,
    pub modified: u64,
    pub created: u64,
}

impl StorageFile {
    fn meta_data(metadata: Metadata) -> (u64, u64, u64, bool) {
        let created = metadata
            .created()
            .unwrap_or_else(|error| {
                error!(error = as_error!(error); "Error getting created time");
                SystemTime::now()
            })
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros();

        let modified = if let Ok(modified) = metadata.modified() {
            modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        } else {
            created
        };
        let directory = metadata.file_type().is_dir();
        let size = metadata.len();
        (created as u64, modified as u64, size, directory)
    }
    pub async fn create_from_entry<S: Into<String>>(
        relative_path: S,
        entry: &DirEntry,
    ) -> Result<Self, StorageError> {
        let metadata = entry.metadata()?;
        let (created, modified, file_size, directory) = Self::meta_data(metadata);

        let mime = mime_guess::from_path(entry.path())
            .first_or_octet_stream()
            .to_string();
        let name = entry.file_name().to_str().unwrap().to_string();
        let file = StorageFile {
            name,
            full_path: relative_path.into(),
            mime,
            directory,
            file_size,
            modified,
            created,
        };
        Ok(file)
    }
    pub async fn create<S: Into<String>>(
        relative_path: S,
        file_location: &PathBuf,
    ) -> Result<Self, StorageError> {
        let metadata = metadata(file_location).await?;
        trace!("Parsing MetaData");
        let (created, modified, file_size, directory) = Self::meta_data(metadata);

        let mime = mime_guess::from_path(file_location)
            .first_or_octet_stream()
            .to_string();
        let name = file_location
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let file = StorageFile {
            name,
            full_path: relative_path.into(),
            mime,
            directory,
            file_size,
            modified,
            created,
        };
        Ok(file)
    }
}
#[derive(Serialize, Clone, Debug)]
pub struct StorageDirectoryResponse {
    pub files: Vec<StorageFile>,
    pub directory: StorageFile,
}
/// The Types of Storage File web responses it can have
pub enum StorageFileResponse {
    /// A Location to a Local File
    File(PathBuf),
    /// A list of StorageFiles. Usually Responded when a directory
    /// First Value is the Information About the directory
    List(StorageDirectoryResponse),
    /// Not Found
    NotFound,
}
/// A Simple trait for handling file List responses
pub trait FileListResponder {
    /// Parses the Accept the header(badly) to decide the Response Type
    fn listing(self, request: &HttpRequest) -> Result<HttpResponse, actix_web::Error>
    where
        Self: Sized,
    {
        if request.method() == Method::HEAD {}
        return if let Some(accept) = request.headers().get(ACCEPT) {
            let x = accept.to_str().map_err(ErrorBadRequest)?;
            if x.contains("application/json") {
                self.json_listing(request)
            } else if x.contains("text/html") {
                self.html_listing(request)
            } else {
                Ok(Self::invalid_accept_type())
            }
        } else {
            self.html_listing(request)
        };
    }
    /// Converts Self into a JSOn based Http Response
    fn json_listing(self, request: &HttpRequest) -> Result<HttpResponse, actix_web::Error>
    where
        Self: Sized;
    /// Converts Self Into a HTML based HTTP Response
    fn html_listing(self, _request: &HttpRequest) -> Result<HttpResponse, actix_web::Error>
    where
        Self: Sized,
    {
        Ok(Self::invalid_accept_type())
    }
    /// For Internal Use
    /// Invalid Data Type
    fn invalid_accept_type() -> HttpResponse {
        HttpResponse::BadRequest().finish()
    }
}
impl FileListResponder for StorageDirectoryResponse {
    fn json_listing(self, request: &HttpRequest) -> Result<HttpResponse, actix_web::Error>
    where
        Self: Sized,
    {
        Ok(HttpResponse::Ok().json(self.files).respond_to(request))
    }
}

impl Responder for StorageFileResponse {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            StorageFileResponse::File(file) => match NamedFile::open(file) {
                Ok(success) => success.respond_to(req),
                Err(error) => {
                    error!("Unable to Respond with File {}", error);
                    HttpResponse::from_error(error).respond_to(req)
                }
            },
            StorageFileResponse::List(list) => match list.listing(req) {
                Ok(response) => response,
                Err(response) => response.error_response(),
            },
            StorageFileResponse::NotFound => {
                HttpResponse::new(StatusCode::NOT_FOUND).respond_to(req)
            }
        }
    }
}
