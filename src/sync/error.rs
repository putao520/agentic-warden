#![allow(dead_code)] // 同步错误模块，部分API函数当前未使用

use crate::error::{errors, AgenticResult, AgenticWardenError, SyncOperation};
use anyhow::Error as AnyError;
use base64::DecodeError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeError;
use std::io;
use url::ParseError;

pub type SyncResult<T> = AgenticResult<T>;

/// Helper namespace that converts legacy sync failures into AgenticWardenError instances.
pub struct SyncError;

impl SyncError {
    pub fn directory_hashing(reason: impl Into<String>) -> AgenticWardenError {
        errors::sync_error(SyncOperation::DirectoryHashing, reason)
    }

    pub fn config_packing(reason: impl Into<String>) -> AgenticWardenError {
        errors::sync_error(SyncOperation::ConfigPacking, reason)
    }

    pub fn google_drive(reason: impl Into<String>) -> AgenticWardenError {
        let message = reason.into();
        if message.contains("User declined") || message.contains("invalid_grant") {
            return AgenticWardenError::Auth {
                message,
                provider: "google_drive".to_string(),
                source: None,
            };
        }
        errors::sync_error(SyncOperation::GoogleDriveRequest, message)
    }

    pub fn sync_config(reason: impl Into<String>) -> AgenticWardenError {
        errors::sync_error(SyncOperation::ConfigLoading, reason)
    }

    pub fn config(reason: impl Into<String>) -> AgenticWardenError {
        AgenticWardenError::Config {
            message: reason.into(),
            source: None,
        }
    }

    pub fn io(err: io::Error) -> AgenticWardenError {
        AgenticWardenError::Filesystem {
            message: format!("Filesystem error during sync: {err}"),
            path: "<sync>".to_string(),
            source: Some(Box::new(err)),
        }
    }

    pub fn json(err: SerdeError) -> AgenticWardenError {
        AgenticWardenError::Config {
            message: format!("Configuration JSON error: {err}"),
            source: Some(Box::new(err)),
        }
    }

    pub fn http(err: ReqwestError) -> AgenticWardenError {
        AgenticWardenError::Network {
            message: format!("Network request failed while talking to Google Drive: {err}"),
            url: None,
            source: Some(Box::new(err)),
        }
    }

    pub fn base64(err: DecodeError) -> AgenticWardenError {
        errors::sync_error(
            SyncOperation::GoogleDriveRequest,
            format!("Failed to decode service response: {err}"),
        )
    }

    pub fn url(err: ParseError) -> AgenticWardenError {
        AgenticWardenError::Validation {
            message: format!("Invalid Google Drive URL: {err}"),
            field: Some("google_drive_url".to_string()),
            value: None,
        }
    }

    pub fn directory_not_found(path: impl Into<String>) -> AgenticWardenError {
        let path = path.into();
        AgenticWardenError::Filesystem {
            message: format!("Directory not found: {path}"),
            path,
            source: None,
        }
    }

    pub fn authentication_required() -> AgenticWardenError {
        AgenticWardenError::Auth {
            message: "Google Drive authentication required".to_string(),
            provider: "google_drive".to_string(),
            source: None,
        }
    }

    pub fn no_changes_detected() -> AgenticWardenError {
        errors::sync_error(
            SyncOperation::Discovery,
            "No local changes detected; skipping upload",
        )
    }

    pub fn upload_failed(reason: impl Into<String>) -> AgenticWardenError {
        errors::sync_error(SyncOperation::Upload, reason)
    }

    pub fn download_failed(reason: impl Into<String>) -> AgenticWardenError {
        errors::sync_error(SyncOperation::Download, reason)
    }

    pub fn general(err: AnyError) -> AgenticWardenError {
        AgenticWardenError::Unknown {
            message: format!("Unexpected sync error: {err}"),
            source: None,
        }
    }

    pub fn not_implemented() -> AgenticWardenError {
        errors::sync_error(
            SyncOperation::Unknown,
            "This sync capability has not been implemented yet",
        )
    }
}
