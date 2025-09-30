use std::io::Cursor;

use google_drive3::{api::File, api::Scope, DriveHub};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveMedia {
    pub file_id: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub web_view_link: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DriveOperationError {
    #[error("drive operation failed: {0}")]
    Backend(String),
    #[error("drive runtime unavailable: {0}")]
    Runtime(String),
}

use std::fmt::Debug;

pub trait DriveGateway: Debug {
    fn list_unit_media(&self, folder_id: &str) -> Result<Vec<DriveMedia>, DriveOperationError>;
    fn create_listing_document(
        &self,
        title: &str,
        html_body: &str,
        parent_folder_id: Option<&str>,
    ) -> Result<String, DriveOperationError>;
}

/// Thin wrapper around the generated google-drive3 client allowing synchronous
/// workflows to interact with Drive without exposing async details.
pub struct GoogleDriveClient<C>
where
    C: google_drive3::common::Connector + Send + Sync + 'static,
{
    hub: DriveHub<C>,
    runtime: Runtime,
}

impl<C> GoogleDriveClient<C>
where
    C: google_drive3::common::Connector + Send + Sync + 'static,
{
    pub fn new(hub: DriveHub<C>, runtime: Runtime) -> Self {
        Self { hub, runtime }
    }

    pub fn with_runtime(hub: DriveHub<C>) -> Result<Self, DriveOperationError> {
        let runtime =
            Runtime::new().map_err(|err| DriveOperationError::Runtime(err.to_string()))?;
        Ok(Self::new(hub, runtime))
    }

    fn map_error<E: std::fmt::Display>(err: E) -> DriveOperationError {
        DriveOperationError::Backend(err.to_string())
    }
}

impl<C> std::fmt::Debug for GoogleDriveClient<C>
where
    C: google_drive3::common::Connector + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoogleDriveClient").finish_non_exhaustive()
    }
}

impl<C> DriveGateway for GoogleDriveClient<C>
where
    C: google_drive3::common::Connector + Send + Sync + 'static,
{
    fn list_unit_media(&self, folder_id: &str) -> Result<Vec<DriveMedia>, DriveOperationError> {
        let folder = folder_id.to_string();
        let result = self.runtime.block_on(async {
            self.hub
                .files()
                .list()
                .q(&format!("'{folder}' in parents and trashed=false"))
                .param("fields", "files(id,name,mimeType,webViewLink)")
                .page_size(25)
                .include_items_from_all_drives(true)
                .supports_all_drives(true)
                .add_scope(Scope::Readonly)
                .doit()
                .await
        });

        let (_, file_list) = result.map_err(GoogleDriveClient::<C>::map_error)?;
        let files = file_list.files.unwrap_or_default();
        Ok(files
            .into_iter()
            .map(|file| DriveMedia {
                file_id: file.id.unwrap_or_default(),
                name: file.name.unwrap_or_else(|| "untitled".to_string()),
                mime_type: file.mime_type,
                web_view_link: file.web_view_link,
            })
            .collect())
    }

    fn create_listing_document(
        &self,
        title: &str,
        html_body: &str,
        parent_folder_id: Option<&str>,
    ) -> Result<String, DriveOperationError> {
        let metadata = File {
            name: Some(title.to_string()),
            mime_type: Some("application/vnd.google-apps.document".to_string()),
            parents: parent_folder_id.map(|parent| vec![parent.to_string()]),
            ..File::default()
        };

        let bytes = html_body.as_bytes().to_vec();
        let cursor = Cursor::new(bytes);

        let result = self.runtime.block_on(async {
            self.hub
                .files()
                .create(metadata)
                .param("fields", "id")
                .supports_all_drives(true)
                .add_scope(Scope::File)
                .upload(cursor, mime::TEXT_HTML)
                .await
        });

        let (_, file) = result.map_err(GoogleDriveClient::<C>::map_error)?;
        Ok(file.id.unwrap_or_default())
    }
}
