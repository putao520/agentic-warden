use super::error::{SyncError, SyncResult};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveFile {
    pub id: String,
    pub name: String,
    pub size: Option<String>,
    pub created_time: String,
    pub modified_time: String,
    pub mime_type: String,
    pub parents: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleDriveConfig {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub base_folder_id: Option<String>,
    pub token_expires_at: Option<i64>, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Debug)]
pub struct GoogleDriveClient {
    pub config: GoogleDriveConfig,
    http_client: reqwest::Client,
}

impl GoogleDriveClient {
    const DRIVE_API_BASE: &'static str = "https://www.googleapis.com/drive/v3";
    const OAUTH_TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    const OAUTH_AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";

    pub fn new(config: GoogleDriveConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Get the path to the auth.json file
    fn auth_file_path() -> SyncResult<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            SyncError::GoogleDriveError("Could not find home directory".to_string())
        })?;
        let warden_dir = home_dir.join(".agentic-warden");

        // Create directory if it doesn't exist
        fs::create_dir_all(&warden_dir).map_err(|e| {
            SyncError::GoogleDriveError(format!("Failed to create warden directory: {}", e))
        })?;

        Ok(warden_dir.join("auth.json"))
    }

    /// Save authentication configuration to auth.json
    pub fn save_auth_config(&self) -> SyncResult<()> {
        let auth_path = Self::auth_file_path()?;
        let content = serde_json::to_string_pretty(&self.config).map_err(|e| {
            SyncError::GoogleDriveError(format!("Failed to serialize auth config: {}", e))
        })?;

        fs::write(&auth_path, content).map_err(|e| {
            SyncError::GoogleDriveError(format!("Failed to write auth file: {}", e))
        })?;

        Ok(())
    }

    /// Load authentication configuration from auth.json
    pub fn load_auth_config() -> SyncResult<Option<GoogleDriveConfig>> {
        let auth_path = Self::auth_file_path()?;

        if !auth_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&auth_path)
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to read auth file: {}", e)))?;

        let config: GoogleDriveConfig = serde_json::from_str(&content).map_err(|e| {
            SyncError::GoogleDriveError(format!("Failed to parse auth file: {}", e))
        })?;

        Ok(Some(config))
    }

    #[allow(dead_code)]
    pub fn from_env() -> SyncResult<Self> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID").map_err(|_| {
            SyncError::GoogleDriveError("GOOGLE_CLIENT_ID environment variable not set".to_string())
        })?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| {
            SyncError::GoogleDriveError(
                "GOOGLE_CLIENT_SECRET environment variable not set".to_string(),
            )
        })?;

        let config = GoogleDriveConfig {
            client_id,
            client_secret,
            access_token: std::env::var("GOOGLE_ACCESS_TOKEN").ok(),
            refresh_token: std::env::var("GOOGLE_REFRESH_TOKEN").ok(),
            base_folder_id: std::env::var("GOOGLE_DRIVE_FOLDER_ID").ok(),
            token_expires_at: std::env::var("GOOGLE_TOKEN_EXPIRES_AT")
                .ok()
                .and_then(|s| s.parse().ok()),
        };

        Ok(Self::new(config))
    }

    pub async fn authenticate(&mut self) -> SyncResult<()> {
        if self.config.access_token.is_some() {
            return Ok(());
        }

        println!("Google Drive authentication required");
        println!("Please follow these steps:");

        // Generate auth URL
        let auth_url = self.generate_auth_url()?;
        println!("\n1. Open this URL in your browser:");
        println!("{}", auth_url);

        println!("\n2. After authorizing, copy the authorization code from the browser");

        // Get authorization code from user
        let auth_code = dialoguer::Input::<String>::new()
            .with_prompt("Enter authorization code")
            .interact_text()
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to read authorization code: {}", e))
            })?;

        // Exchange code for tokens
        self.exchange_code_for_tokens(&auth_code).await?;

        println!("Authentication successful!");
        Ok(())
    }

    pub fn generate_auth_url(&self) -> SyncResult<String> {
        let mut url = Url::parse(Self::OAUTH_AUTH_URL)
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to parse auth URL: {}", e)))?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", "urn:ietf:wg:oauth:2.0:oob")
            .append_pair("response_type", "code")
            .append_pair("scope", "https://www.googleapis.com/auth/drive.file")
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        Ok(url.to_string())
    }

    async fn exchange_code_for_tokens(&mut self, code: &str) -> SyncResult<()> {
        let params = [
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("code", code.to_string()),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob".to_string()),
        ];

        let response: OAuthTokenResponse = self
            .http_client
            .post(Self::OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to exchange authorization code: {}", e))
            })?
            .json()
            .await
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to parse token response: {}", e))
            })?;

        self.config.access_token = Some(response.access_token);
        self.config.refresh_token = response
            .refresh_token
            .or_else(|| self.config.refresh_token.clone());

        // Calculate token expiry time (subtract 5 minutes for safety margin)
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + (response.expires_in as i64) - 300; // 5 minutes buffer
        self.config.token_expires_at = Some(expires_at);

        // Save authentication configuration to auth.json
        self.save_auth_config()?;

        Ok(())
    }

    async fn ensure_valid_access_token(&mut self) -> SyncResult<()> {
        if self.config.access_token.is_none() {
            self.authenticate().await?;
            return Ok(());
        }

        // Check if token is expired or will expire soon
        if self.is_token_expired()
            && let Err(e) = self.refresh_access_token().await
        {
            println!("Failed to refresh token: {}", e);
            println!("Re-authenticating...");
            self.authenticate().await?;
        }

        Ok(())
    }

    /// Check if the access token is expired or will expire soon
    pub fn is_token_expired(&self) -> bool {
        if let Some(expires_at) = self.config.token_expires_at {
            let now = chrono::Utc::now().timestamp();
            now >= expires_at
        } else {
            // If we don't have expiry info, assume token might be expired
            true
        }
    }

    /// Refresh the access token using the refresh token
    async fn refresh_access_token(&mut self) -> SyncResult<()> {
        let refresh_token =
            self.config.refresh_token.as_ref().ok_or_else(|| {
                SyncError::GoogleDriveError("No refresh token available".to_string())
            })?;

        let params = [
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
            ("refresh_token", refresh_token.clone()),
            ("grant_type", "refresh_token".to_string()),
        ];

        let response: OAuthTokenResponse = self
            .http_client
            .post(Self::OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to refresh token: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to parse refresh response: {}", e))
            })?;

        self.config.access_token = Some(response.access_token);

        // Update refresh token if a new one was provided
        if let Some(new_refresh_token) = response.refresh_token {
            self.config.refresh_token = Some(new_refresh_token);
        }

        // Calculate new token expiry time (subtract 5 minutes for safety margin)
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + (response.expires_in as i64) - 300; // 5 minutes buffer
        self.config.token_expires_at = Some(expires_at);

        // Save updated authentication configuration
        self.save_auth_config()?;

        println!("Token refreshed successfully");
        Ok(())
    }

    pub async fn ensure_folder_exists(&mut self, folder_name: &str) -> SyncResult<String> {
        self.ensure_valid_access_token().await?;

        // First, ensure base folder exists
        let base_folder_id = if let Some(base_id) = &self.config.base_folder_id {
            base_id.clone()
        } else {
            let base_id = self.create_or_find_folder("agentic-warden", None).await?;
            self.config.base_folder_id = Some(base_id.clone());
            base_id
        };

        // Create or find the specific folder
        self.create_or_find_folder(folder_name, Some(&base_folder_id))
            .await
    }

    async fn create_or_find_folder(
        &mut self,
        folder_name: &str,
        parent_id: Option<&str>,
    ) -> SyncResult<String> {
        self.ensure_valid_access_token().await?;

        // Search for existing folder
        let query = format!(
            "name='{}' and mimeType='application/vnd.google-apps.folder'",
            folder_name
        );

        if let Some(parent) = parent_id {
            let search_url = format!(
                "{}/files?q={} and parents in '{}'&fields=files(id,name)",
                Self::DRIVE_API_BASE,
                urlencoding::encode(&query),
                parent
            );

            let response: serde_json::Value = self
                .http_client
                .get(&search_url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.get_access_token()?),
                )
                .send()
                .await
                .map_err(|e| {
                    SyncError::GoogleDriveError(format!("Failed to search for folder: {}", e))
                })?
                .json()
                .await
                .map_err(|e| {
                    SyncError::GoogleDriveError(format!("Failed to parse search response: {}", e))
                })?;

            if let Some(files) = response["files"].as_array()
                && !files.is_empty()
                && let Some(id) = files[0]["id"].as_str()
            {
                return Ok(id.to_string());
            }
        }

        // Create new folder
        let mut folder_data = serde_json::json!({
            "name": folder_name,
            "mimeType": "application/vnd.google-apps.folder"
        });

        if let Some(parent) = parent_id {
            folder_data["parents"] = serde_json::json!([parent]);
        }

        let response: serde_json::Value = self
            .http_client
            .post(format!("{}/files", Self::DRIVE_API_BASE))
            .header(
                "Authorization",
                format!("Bearer {}", self.get_access_token()?),
            )
            .json(&folder_data)
            .send()
            .await
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to create folder: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                SyncError::GoogleDriveError(format!(
                    "Failed to parse folder creation response: {}",
                    e
                ))
            })?;

        let folder_id = response["id"].as_str().ok_or_else(|| {
            SyncError::GoogleDriveError("No folder ID in creation response".to_string())
        })?;

        Ok(folder_id.to_string())
    }

    pub async fn upload_file(
        &mut self,
        file_path: &Path,
        folder_id: &str,
    ) -> SyncResult<GoogleDriveFile> {
        self.ensure_valid_access_token().await?;
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SyncError::GoogleDriveError("Invalid file name".to_string()))?
            .to_string();

        let file_content = std::fs::read(file_path)
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to read file: {}", e)))?;

        // Create metadata
        let metadata = serde_json::json!({
            "name": file_name,
            "parents": [folder_id]
        });

        // Create multipart form
        let metadata_part = Part::text(metadata.to_string())
            .file_name("metadata")
            .mime_str("application/json")
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to create metadata part: {}", e))
            })?;

        let file_part = Part::bytes(file_content)
            .file_name(file_name)
            .mime_str("application/gzip")
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to create file part: {}", e))
            })?;

        let form = Form::new()
            .part("metadata", metadata_part)
            .part("file", file_part);

        let upload_url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,size,createdTime,modifiedTime,mimeType,parents".to_string();

        let response: GoogleDriveFile = self
            .http_client
            .post(&upload_url)
            .header(
                "Authorization",
                format!("Bearer {}", self.get_access_token()?),
            )
            .multipart(form)
            .send()
            .await
            .map_err(|e| SyncError::UploadFailed(format!("Upload failed: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to parse upload response: {}", e))
            })?;

        Ok(response)
    }

    pub async fn download_file(&mut self, file_id: &str, output_path: &str) -> SyncResult<()> {
        self.ensure_valid_access_token().await?;

        let download_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file_id
        );

        let response = self
            .http_client
            .get(&download_url)
            .header(
                "Authorization",
                format!("Bearer {}", self.get_access_token()?),
            )
            .send()
            .await
            .map_err(|e| SyncError::DownloadFailed(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SyncError::DownloadFailed(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let content = response
            .bytes()
            .await
            .map_err(|e| SyncError::DownloadFailed(format!("Failed to read response: {}", e)))?;

        std::fs::write(output_path, content).map_err(|e| {
            SyncError::GoogleDriveError(format!("Failed to write downloaded file: {}", e))
        })?;

        Ok(())
    }

    pub async fn list_files(&mut self, folder_id: &str) -> SyncResult<Vec<GoogleDriveFile>> {
        self.ensure_valid_access_token().await?;

        let query = format!("parents in '{}'", folder_id);
        let url = format!(
            "{}/files?q={}&fields=files(id,name,size,createdTime,modifiedTime,mimeType,parents)",
            Self::DRIVE_API_BASE,
            urlencoding::encode(&query)
        );

        let response: serde_json::Value = self
            .http_client
            .get(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.get_access_token()?),
            )
            .send()
            .await
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to list files: {}", e)))?
            .json()
            .await
            .map_err(|e| {
                SyncError::GoogleDriveError(format!("Failed to parse list response: {}", e))
            })?;

        let files = response["files"].as_array().ok_or_else(|| {
            SyncError::GoogleDriveError("Invalid files list in response".to_string())
        })?;

        let mut result = Vec::new();
        for file in files {
            if let Ok(drive_file) = serde_json::from_value::<GoogleDriveFile>(file.clone()) {
                result.push(drive_file);
            }
        }

        Ok(result)
    }

    pub async fn find_file(
        &mut self,
        folder_id: &str,
        file_name: &str,
    ) -> SyncResult<Option<GoogleDriveFile>> {
        let files = self.list_files(folder_id).await?;
        Ok(files.into_iter().find(|f| f.name == file_name))
    }

    pub async fn delete_file(&mut self, file_id: &str) -> SyncResult<()> {
        self.ensure_valid_access_token().await?;

        let url = format!("{}/files/{}", Self::DRIVE_API_BASE, file_id);

        self.http_client
            .delete(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.get_access_token()?),
            )
            .send()
            .await
            .map_err(|e| SyncError::GoogleDriveError(format!("Failed to delete file: {}", e)))?;

        Ok(())
    }

    pub fn get_access_token(&self) -> SyncResult<&String> {
        self.config
            .access_token
            .as_ref()
            .ok_or_else(|| SyncError::AuthenticationRequired)
    }

    #[allow(dead_code)]
    pub fn get_config(&self) -> &GoogleDriveConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_auth_url() {
        let config = GoogleDriveConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
            base_folder_id: None,
            token_expires_at: None,
        };

        let client = GoogleDriveClient::new(config);
        let url = client.generate_auth_url().unwrap();
        assert!(url.contains("accounts.google.com"));
        assert!(url.contains("test_client_id"));
    }
}
