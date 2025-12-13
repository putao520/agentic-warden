// Google Drive Service - Using OAuth with HTTP requests
// This module provides Google Drive operations using OAuth authentication and HTTP API calls

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use mime_guess::from_path;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

use super::oauth_client::OAuthClient;

/// Google Drive File Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub size: Option<i64>,
    pub mime_type: String,
    pub created_time: Option<DateTime<Utc>>,
    pub modified_time: Option<DateTime<Utc>>,
    pub parents: Option<Vec<String>>,
    pub web_view_link: Option<String>,
    pub web_content_link: Option<String>,
}

/// Google Drive API response structures
#[derive(Debug, Deserialize)]
struct DriveFileResponse {
    id: String,
    name: String,
    size: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(rename = "createdTime")]
    created_time: Option<String>,
    #[serde(rename = "modifiedTime")]
    modified_time: Option<String>,
    parents: Option<Vec<String>>,
    #[serde(rename = "webViewLink")]
    web_view_link: Option<String>,
    #[serde(rename = "webContentLink")]
    web_content_link: Option<String>,
}

impl From<DriveFileResponse> for DriveFile {
    fn from(response: DriveFileResponse) -> Self {
        Self {
            id: response.id,
            name: response.name,
            size: response.size.and_then(|s| s.parse().ok()),
            mime_type: response.mime_type,
            created_time: response.created_time.and_then(|dt| {
                DateTime::parse_from_rfc3339(&dt)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }),
            modified_time: response.modified_time.and_then(|dt| {
                DateTime::parse_from_rfc3339(&dt)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }),
            parents: response.parents,
            web_view_link: response.web_view_link,
            web_content_link: response.web_content_link,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DriveFileListResponse {
    files: Option<Vec<DriveFileResponse>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

/// Google Drive Service using OAuth and HTTP requests
#[derive(Debug, Clone)]
pub struct GoogleDriveService {
    oauth_client: OAuthClient,
    http_client: reqwest::Client,
}

#[allow(dead_code)]
impl GoogleDriveService {
    const DRIVE_API_BASE: &'static str = "https://www.googleapis.com/drive/v3";

    /// Create new Google Drive service
    pub async fn new(oauth_client: OAuthClient) -> Result<Self> {
        info!("Initializing Google Drive service with OAuth");

        Ok(Self {
            oauth_client,
            http_client: reqwest::Client::new(),
        })
    }

    /// Get access token for authenticated requests
    async fn get_access_token(&mut self) -> Result<String> {
        self.oauth_client
            .access_token()
            .await
            .context("Failed to get access token")
    }

    /// Create or find folder
    pub async fn create_or_find_folder(
        &mut self,
        folder_name: &str,
        parent_id: Option<&str>,
    ) -> Result<String> {
        info!("Creating or finding folder: {}", folder_name);

        // First try to find existing folder
        if let Some(folder_id) = self.find_folder(folder_name, parent_id).await? {
            info!("Found existing folder: {} (ID: {})", folder_name, folder_id);
            return Ok(folder_id);
        }

        // Create new folder
        info!("Creating new folder: {}", folder_name);

        let access_token = self.get_access_token().await?;

        let mut folder_metadata = serde_json::json!({
            "name": folder_name,
            "mimeType": "application/vnd.google-apps.folder"
        });

        if let Some(parent) = parent_id {
            folder_metadata["parents"] = serde_json::json!([parent]);
        }

        let response = self
            .http_client
            .post(format!("{}/files", Self::DRIVE_API_BASE))
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&folder_metadata)
            .query(&[("fields", "id,name")])
            .send()
            .await
            .context("Failed to create folder")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to create folder: {}", error_text));
        }

        let file_response: DriveFileResponse = response
            .json()
            .await
            .context("Failed to parse folder creation response")?;

        let folder_id = file_response.id;

        info!(
            "Successfully created folder: {} (ID: {})",
            folder_name, folder_id
        );
        Ok(folder_id)
    }

    /// Find folder by name
    pub async fn find_folder(
        &mut self,
        folder_name: &str,
        parent_id: Option<&str>,
    ) -> Result<Option<String>> {
        debug!("Searching for folder: {}", folder_name);

        let mut query = format!(
            "name='{}' and mimeType='application/vnd.google-apps.folder' and trashed=false",
            folder_name
        );

        if let Some(parent) = parent_id {
            query.push_str(&format!(" and parents in '{}'", parent));
        }

        let access_token = self.get_access_token().await?;

        let response = self
            .http_client
            .get(format!("{}/files", Self::DRIVE_API_BASE))
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("q", query.as_str()),
                ("fields", "files(id,name,parents)"),
                ("pageSize", "10"),
            ])
            .send()
            .await
            .context("Failed to search for folder")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to search for folder: {}", error_text));
        }

        let list_response: DriveFileListResponse = response
            .json()
            .await
            .context("Failed to parse search response")?;

        if let Some(files) = list_response.files {
            if !files.is_empty() {
                let folder_id = files[0].id.clone();
                debug!("Found folder: {} (ID: {})", folder_name, folder_id);
                return Ok(Some(folder_id));
            }
        }

        debug!("Folder not found: {}", folder_name);
        Ok(None)
    }

    /// Upload file content
    pub async fn upload_file_content(
        &mut self,
        file_name: &str,
        content: Vec<u8>,
        folder_id: Option<&str>,
    ) -> Result<String> {
        info!("Uploading file: {}", file_name);

        let mime_type = from_path(file_name).first_or_octet_stream().to_string();

        let access_token = self.get_access_token().await?;

        // Create metadata
        let mut metadata = serde_json::json!({
            "name": file_name,
            "mimeType": mime_type
        });

        if let Some(folder) = folder_id {
            metadata["parents"] = serde_json::json!([folder]);
        }

        // Create multipart form
        let metadata_part = Part::text(metadata.to_string())
            .file_name("metadata")
            .mime_str("application/json")?;

        let file_part = Part::bytes(content)
            .file_name(file_name.to_string())
            .mime_str(&mime_type)?;

        let form = Form::new()
            .part("metadata", metadata_part)
            .part("file", file_part);

        let upload_url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,size,createdTime,modifiedTime,mimeType,parents,webViewLink,webContentLink";

        let response = self
            .http_client
            .post(upload_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .multipart(form)
            .send()
            .await
            .context("Failed to upload file")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to upload file: {}", error_text));
        }

        let file_response: DriveFileResponse = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        let file_id = file_response.id;

        info!(
            "Successfully uploaded file: {} (ID: {})",
            file_name, file_id
        );
        Ok(file_id)
    }

    /// Upload file from local path
    pub async fn upload_file(
        &mut self,
        file_path: &Path,
        folder_id: Option<&str>,
    ) -> Result<DriveFile> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid file name"))?;

        info!("Uploading file from path: {:?}", file_path);

        // Read file content as bytes to preserve binary archives
        let content = fs::read(file_path).context("Failed to read file content")?;

        let file_id = self
            .upload_file_content(file_name, content, folder_id)
            .await?;

        // Get file information
        self.get_file_info(&file_id).await
    }

    /// Download file content
    pub async fn download_file_content(&mut self, file_id: &str) -> Result<Vec<u8>> {
        info!("Downloading file content: {}", file_id);

        let access_token = self.get_access_token().await?;

        let download_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file_id
        );

        let response = self
            .http_client
            .get(&download_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .context("Failed to download file")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to download file: {}", error_text));
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read download response")?;

        info!(
            "Successfully downloaded file content: {} bytes",
            bytes.len()
        );
        Ok(bytes.to_vec())
    }

    /// Download file to local path
    pub async fn download_file(&mut self, file_id: &str, output_path: &Path) -> Result<()> {
        info!("Downloading file to: {:?}", output_path);

        let content = self.download_file_content(file_id).await?;

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).context("Failed to create output directory")?;
        }

        fs::write(output_path, content).context("Failed to write downloaded file")?;

        info!("Successfully downloaded file to: {:?}", output_path);
        Ok(())
    }

    /// Get file information
    pub async fn get_file_info(&mut self, file_id: &str) -> Result<DriveFile> {
        debug!("Getting file info: {}", file_id);

        let access_token = self.get_access_token().await?;

        let response = self
            .http_client
            .get(format!("{}/files/{}", Self::DRIVE_API_BASE, file_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[(
                "fields",
                "id,name,size,createdTime,modifiedTime,mimeType,parents,webViewLink,webContentLink",
            )])
            .send()
            .await
            .context("Failed to get file info")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to get file info: {}", error_text));
        }

        let file_response: DriveFileResponse = response
            .json()
            .await
            .context("Failed to parse file info response")?;

        let drive_file = DriveFile::from(file_response);
        debug!(
            "Got file info: {} ({} bytes)",
            drive_file.name,
            drive_file.size.unwrap_or(0)
        );
        Ok(drive_file)
    }

    /// List files in folder
    pub async fn list_folder_files(&mut self, folder_id: &str) -> Result<Vec<DriveFile>> {
        info!("Listing files in folder: {}", folder_id);

        let mut files = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let access_token = self.get_access_token().await?;

            let mut request = self.http_client
                .get(format!("{}/files", Self::DRIVE_API_BASE))
                .header("Authorization", format!("Bearer {}", access_token))
                .query(&[
                    ("q", format!("parents in '{}' and trashed=false", folder_id).as_str()),
                    ("fields", "files(id,name,size,createdTime,modifiedTime,mimeType,parents,webViewLink,webContentLink),nextPageToken"),
                    ("pageSize", "100")
                ]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = request.send().await.context("Failed to list files")?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .context("Failed to read error response")?;
                return Err(anyhow!("Failed to list files: {}", error_text));
            }

            let list_response: DriveFileListResponse = response
                .json()
                .await
                .context("Failed to parse list response")?;

            if let Some(file_list) = list_response.files {
                for file in file_list {
                    files.push(DriveFile::from(file));
                }
            }

            page_token = list_response.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        info!("Found {} files in folder {}", files.len(), folder_id);
        Ok(files)
    }

    /// Search for files
    pub async fn search_files(&mut self, query: &str) -> Result<Vec<DriveFile>> {
        info!("Searching files with query: {}", query);

        let mut files = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let access_token = self.get_access_token().await?;

            let mut request = self.http_client
                .get(format!("{}/files", Self::DRIVE_API_BASE))
                .header("Authorization", format!("Bearer {}", access_token))
                .query(&[
                    ("q", format!("{} and trashed=false", query).as_str()),
                    ("fields", "files(id,name,size,createdTime,modifiedTime,mimeType,parents,webViewLink,webContentLink),nextPageToken"),
                    ("pageSize", "100")
                ]);

            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }

            let response = request.send().await.context("Failed to search files")?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .context("Failed to read error response")?;
                return Err(anyhow!("Failed to search files: {}", error_text));
            }

            let list_response: DriveFileListResponse = response
                .json()
                .await
                .context("Failed to parse search response")?;

            if let Some(file_list) = list_response.files {
                for file in file_list {
                    files.push(DriveFile::from(file));
                }
            }

            page_token = list_response.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        info!("Found {} files matching query: {}", files.len(), query);
        Ok(files)
    }

    /// Delete file
    pub async fn delete_file(&mut self, file_id: &str) -> Result<()> {
        info!("Deleting file: {}", file_id);

        let access_token = self.get_access_token().await?;

        let response = self
            .http_client
            .delete(format!("{}/files/{}", Self::DRIVE_API_BASE, file_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .context("Failed to delete file")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to delete file: {}", error_text));
        }

        info!("Successfully deleted file: {}", file_id);
        Ok(())
    }

    /// Create folder
    pub async fn create_folder(&mut self, folder_name: &str) -> Result<String> {
        self.create_or_find_folder(folder_name, None).await
    }

    /// Create folder in parent
    pub async fn create_folder_in_parent(
        &mut self,
        folder_name: &str,
        parent_id: &str,
    ) -> Result<String> {
        self.create_or_find_folder(folder_name, Some(parent_id))
            .await
    }

    /// Update file content
    pub async fn update_file_content(&mut self, file_id: &str, content: &str) -> Result<()> {
        info!("Updating file content: {}", file_id);

        // First get file info to preserve metadata
        let file_info = self.get_file_info(file_id).await?;
        let mime_type = file_info.mime_type;

        let access_token = self.get_access_token().await?;

        // Create multipart form for update
        let metadata = serde_json::json!({
            "name": file_info.name,
            "mimeType": mime_type
        });

        let metadata_part = Part::text(metadata.to_string())
            .file_name("metadata")
            .mime_str("application/json")?;

        let file_part = Part::bytes(content.as_bytes().to_vec())
            .file_name(file_info.name.clone())
            .mime_str(&mime_type)?;

        let form = Form::new()
            .part("metadata", metadata_part)
            .part("file", file_part);

        let upload_url = format!(
            "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=multipart&fields=id,name,size,modifiedTime",
            file_id
        );

        let response = self
            .http_client
            .patch(&upload_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .multipart(form)
            .send()
            .await
            .context("Failed to update file")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to update file: {}", error_text));
        }

        info!("Successfully updated file: {}", file_id);
        Ok(())
    }

    /// Copy file
    pub async fn copy_file(
        &mut self,
        file_id: &str,
        new_name: &str,
        destination_folder_id: Option<&str>,
    ) -> Result<String> {
        info!("Copying file {} as {}", file_id, new_name);

        let access_token = self.get_access_token().await?;

        let mut metadata = serde_json::json!({
            "name": new_name
        });

        if let Some(folder) = destination_folder_id {
            metadata["parents"] = serde_json::json!([folder]);
        }

        let response = self
            .http_client
            .post(format!("{}/files/{}/copy", Self::DRIVE_API_BASE, file_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&metadata)
            .query(&[(
                "fields",
                "id,name,size,createdTime,modifiedTime,mimeType,parents,webViewLink,webContentLink",
            )])
            .send()
            .await
            .context("Failed to copy file")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to copy file: {}", error_text));
        }

        let file_response: DriveFileResponse = response
            .json()
            .await
            .context("Failed to parse copy response")?;

        let new_file_id = file_response.id;

        info!(
            "Successfully copied file: {} -> {} (ID: {})",
            file_id, new_name, new_file_id
        );
        Ok(new_file_id)
    }

    /// Move file to different folder
    pub async fn move_file(&mut self, file_id: &str, new_parent_id: &str) -> Result<()> {
        info!("Moving file {} to folder {}", file_id, new_parent_id);

        // Get current file info to preserve other parents
        let file_info = self.get_file_info(file_id).await?;
        let current_parents = file_info.parents.unwrap_or_default();

        let access_token = self.get_access_token().await?;

        let metadata = serde_json::json!({
            "name": file_info.name
        });

        let response = self
            .http_client
            .patch(format!("{}/files/{}", Self::DRIVE_API_BASE, file_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&metadata)
            .query(&[
                ("addParents", new_parent_id),
                ("removeParents", &current_parents.join(",")),
                ("fields", "id,name,parents"),
            ])
            .send()
            .await
            .context("Failed to move file")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to move file: {}", error_text));
        }

        info!(
            "Successfully moved file: {} to folder: {}",
            file_id, new_parent_id
        );
        Ok(())
    }

    /// Get file metadata only (without downloading content)
    pub async fn get_file_metadata(&mut self, file_id: &str) -> Result<DriveFile> {
        debug!("Getting file metadata: {}", file_id);

        let access_token = self.get_access_token().await?;

        let response = self
            .http_client
            .get(format!("{}/files/{}", Self::DRIVE_API_BASE, file_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[(
                "fields",
                "id,name,size,createdTime,modifiedTime,mimeType,parents,webViewLink,webContentLink",
            )])
            .send()
            .await
            .context("Failed to get file metadata")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read error response")?;
            return Err(anyhow!("Failed to get file metadata: {}", error_text));
        }

        let file_response: DriveFileResponse = response
            .json()
            .await
            .context("Failed to parse file metadata response")?;

        Ok(DriveFile::from(file_response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_file_conversion() {
        let response = DriveFileResponse {
            id: "test_id".to_string(),
            name: "test_file.txt".to_string(),
            size: Some("1024".to_string()),
            mime_type: "text/plain".to_string(),
            created_time: Some("2024-01-01T00:00:00.000Z".to_string()),
            modified_time: Some("2024-01-01T01:00:00.000Z".to_string()),
            parents: Some(vec!["parent_id".to_string()]),
            web_view_link: Some("https://drive.google.com/file/d/test_id/view".to_string()),
            web_content_link: Some("https://drive.google.com/uc?id=test_id".to_string()),
        };

        let drive_file = DriveFile::from(response);
        assert_eq!(drive_file.id, "test_id");
        assert_eq!(drive_file.name, "test_file.txt");
        assert_eq!(drive_file.size, Some(1024));
        assert_eq!(drive_file.mime_type, "text/plain");
    }

    // Note: Integration tests with real Google Drive API require OAuth setup
    // These would be in the separate integration test files
}
