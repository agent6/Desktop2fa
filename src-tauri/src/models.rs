use serde::{Deserialize, Serialize};

use crate::totp_logic::AlgorithmName;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountMetadata {
    pub id: String,
    pub service_name: String,
    pub issuer: Option<String>,
    pub account_label: Option<String>,
    pub digits: u32,
    pub period: u64,
    pub algorithm: AlgorithmName,
    pub icon: Option<String>,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountPayload {
    pub service_name: String,
    pub issuer: Option<String>,
    pub account_label: Option<String>,
    pub secret: Option<String>,
    pub digits: u32,
    pub period: u64,
    pub algorithm: AlgorithmName,
    pub icon: Option<String>,
    pub otp_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeView {
    pub id: String,
    pub service_name: String,
    pub account_label: Option<String>,
    pub formatted_code: String,
    pub raw_code: String,
    pub seconds_remaining: u64,
    pub period: u64,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AccountEditorMode {
    Create,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountEditorContext {
    pub mode: AccountEditorMode,
    pub account_id: Option<String>,
}

impl Default for AccountEditorContext {
    fn default() -> Self {
        Self {
            mode: AccountEditorMode::Create,
            account_id: None,
        }
    }
}
