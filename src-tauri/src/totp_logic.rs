use std::time::{SystemTime, UNIX_EPOCH};

use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use url::Url;

use crate::app_error::{AppError, AppResult};
use crate::models::{AccountMetadata, AccountPayload};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlgorithmName {
    SHA1,
    SHA256,
    SHA512,
}

#[derive(Debug, Clone)]
pub struct NormalizedPayload {
    pub service_name: String,
    pub issuer: Option<String>,
    pub account_label: Option<String>,
    pub secret: Option<String>,
    pub digits: u32,
    pub period: u64,
    pub algorithm: AlgorithmName,
    pub icon: Option<String>,
}

impl Default for AlgorithmName {
    fn default() -> Self {
        Self::SHA1
    }
}

pub fn normalize_payload(
    payload: &AccountPayload,
    existing: Option<&AccountMetadata>,
) -> AppResult<NormalizedPayload> {
    let otp_uri = payload.otp_uri.as_deref().unwrap_or_default().trim();
    if !otp_uri.is_empty() {
        return normalize_otp_uri_payload(otp_uri, payload.icon.clone(), existing);
    }

    let service_name = require_text(&payload.service_name, "Service name is required")?;
    let issuer = optional_text(payload.issuer.clone());
    let account_label = optional_text(payload.account_label.clone());
    let icon = optional_text(payload.icon.clone());

    validate_digits(payload.digits)?;
    validate_period(payload.period)?;

    let secret = match payload.secret.clone().map(|secret| secret.trim().to_string()) {
        Some(secret) if !secret.is_empty() => Some(normalize_secret(&secret)?),
        _ => None,
    };

    if existing.is_none() && secret.is_none() {
        return Err(AppError::Validation("Secret is required".into()));
    }

    Ok(NormalizedPayload {
        service_name,
        issuer,
        account_label,
        secret,
        digits: payload.digits,
        period: payload.period,
        algorithm: payload.algorithm,
        icon,
    })
}

fn normalize_otp_uri_payload(
    otp_uri: &str,
    icon: Option<String>,
    existing: Option<&AccountMetadata>,
) -> AppResult<NormalizedPayload> {
    let uri = Url::parse(otp_uri)
        .map_err(|_| AppError::Validation("Invalid OTP URI".into()))?;
    if uri.scheme() != "otpauth" {
        return Err(AppError::Validation("OTP URI must start with otpauth://".into()));
    }
    if uri.host_str() != Some("totp") {
        return Err(AppError::Validation("Only otpauth://totp URIs are supported".into()));
    }

    let label = uri.path().trim_start_matches('/');
    let decoded_label = urlencoding::decode(label)
        .map_err(|_| AppError::Validation("Invalid OTP URI label".into()))?
        .to_string();
    let (label_issuer, label_account) = split_label(&decoded_label);

    let mut secret = None;
    let mut issuer = None;
    let mut digits = existing.map(|account| account.digits).unwrap_or(6);
    let mut period = existing.map(|account| account.period).unwrap_or(30);
    let mut algorithm = existing.map(|account| account.algorithm).unwrap_or_default();

    for (key, value) in uri.query_pairs() {
        match key.as_ref() {
            "secret" => secret = Some(normalize_secret(&value)?),
            "issuer" => issuer = Some(value.to_string()),
            "digits" => {
                digits = value
                    .parse::<u32>()
                    .map_err(|_| AppError::Validation("OTP URI digits must be numeric".into()))?;
            }
            "period" => {
                period = value
                    .parse::<u64>()
                    .map_err(|_| AppError::Validation("OTP URI period must be numeric".into()))?;
            }
            "algorithm" => {
                algorithm = parse_algorithm(&value)?;
            }
            _ => {}
        }
    }

    let service_name = issuer
        .clone()
        .or(label_issuer)
        .unwrap_or_else(|| "Imported account".into());
    let account_label = label_account;
    validate_digits(digits)?;
    validate_period(period)?;

    Ok(NormalizedPayload {
        service_name,
        issuer,
        account_label,
        secret,
        digits,
        period,
        algorithm,
        icon: optional_text(icon),
    })
}

fn split_label(label: &str) -> (Option<String>, Option<String>) {
    if let Some((issuer, account_label)) = label.split_once(':') {
        (
            optional_text(Some(issuer.into())),
            optional_text(Some(account_label.into())),
        )
    } else {
        (None, optional_text(Some(label.into())))
    }
}

pub fn generate_code(account: &AccountMetadata, secret: &str) -> AppResult<String> {
    let secret_bytes = decode_secret(secret)?;
    let counter = current_unix_time() / account.period;
    generate_code_for_counter(account.algorithm, account.digits, &secret_bytes, counter)
}

pub fn format_code(code: &str) -> String {
    let midpoint = code.len() / 2;
    format!("{} {}", &code[..midpoint], &code[midpoint..])
}

pub fn seconds_remaining(period: u64) -> u64 {
    let current = current_unix_time();
    let remainder = current % period;
    if remainder == 0 {
        period
    } else {
        period - remainder
    }
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_secs()
}

fn normalize_secret(secret: &str) -> AppResult<String> {
    let normalized = secret
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-')
        .collect::<String>()
        .trim_end_matches('=')
        .to_ascii_uppercase();
    decode_secret(&normalized)?;
    Ok(normalized)
}

fn generate_code_for_counter(
    algorithm: AlgorithmName,
    digits: u32,
    secret: &[u8],
    counter: u64,
) -> AppResult<String> {
    let message = counter.to_be_bytes();
    let truncated = match algorithm {
        AlgorithmName::SHA1 => truncate_hmac_sha1(secret, &message)?,
        AlgorithmName::SHA256 => truncate_hmac_sha256(secret, &message)?,
        AlgorithmName::SHA512 => truncate_hmac_sha512(secret, &message)?,
    };

    let modulo = 10_u32.pow(digits);
    Ok(format!("{:0width$}", truncated % modulo, width = digits as usize))
}

fn dynamic_truncate(digest: &[u8]) -> u32 {
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    ((digest[offset] & 0x7f) as u32) << 24
        | (digest[offset + 1] as u32) << 16
        | (digest[offset + 2] as u32) << 8
        | (digest[offset + 3] as u32)
}

fn truncate_hmac_sha1(secret: &[u8], message: &[u8]) -> AppResult<u32> {
    let mut mac =
        Hmac::<Sha1>::new_from_slice(secret).map_err(|error| AppError::Other(error.to_string()))?;
    mac.update(message);
    Ok(dynamic_truncate(&mac.finalize().into_bytes()))
}

fn truncate_hmac_sha256(secret: &[u8], message: &[u8]) -> AppResult<u32> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|error| AppError::Other(error.to_string()))?;
    mac.update(message);
    Ok(dynamic_truncate(&mac.finalize().into_bytes()))
}

fn truncate_hmac_sha512(secret: &[u8], message: &[u8]) -> AppResult<u32> {
    let mut mac = Hmac::<Sha512>::new_from_slice(secret)
        .map_err(|error| AppError::Other(error.to_string()))?;
    mac.update(message);
    Ok(dynamic_truncate(&mac.finalize().into_bytes()))
}

fn decode_secret(secret: &str) -> AppResult<Vec<u8>> {
    BASE32_NOPAD
        .decode(secret.as_bytes())
        .map_err(|_| AppError::Validation("Secret must be valid base32".into()))
}

fn require_text(value: &str, message: &str) -> AppResult<String> {
    let text = value.trim();
    if text.is_empty() {
        return Err(AppError::Validation(message.into()));
    }
    Ok(text.into())
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn parse_algorithm(value: &str) -> AppResult<AlgorithmName> {
    match value.to_ascii_uppercase().as_str() {
        "SHA1" => Ok(AlgorithmName::SHA1),
        "SHA256" => Ok(AlgorithmName::SHA256),
        "SHA512" => Ok(AlgorithmName::SHA512),
        _ => Err(AppError::Validation(
            "Algorithm must be SHA1, SHA256, or SHA512".into(),
        )),
    }
}

fn validate_digits(digits: u32) -> AppResult<()> {
    if !(6..=8).contains(&digits) {
        return Err(AppError::Validation("Digits must be between 6 and 8".into()));
    }
    Ok(())
}

fn validate_period(period: u64) -> AppResult<()> {
    if period < 15 || period > 120 {
        return Err(AppError::Validation(
            "Period must be between 15 and 120 seconds".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account(algorithm: AlgorithmName, digits: u32, period: u64) -> AccountMetadata {
        AccountMetadata {
            id: "1".into(),
            service_name: "GitHub".into(),
            issuer: Some("GitHub".into()),
            account_label: Some("user@example.com".into()),
            digits,
            period,
            algorithm,
            icon: None,
            sort_order: 0,
        }
    }

    #[test]
    fn formats_code_for_display() {
        assert_eq!(format_code("123456"), "123 456");
    }

    #[test]
    fn normalizes_secret_and_rejects_invalid_base32() {
        assert_eq!(
            normalize_secret("jbsw y3dp-ehpk 3pxp")
                .expect("secret should normalize"),
            "JBSWY3DPEHPK3PXP"
        );
        assert!(normalize_secret("not-base32!").is_err());
    }

    #[test]
    fn parses_otp_uri() {
        let payload = AccountPayload {
            service_name: String::new(),
            issuer: None,
            account_label: None,
            secret: None,
            digits: 6,
            period: 30,
            algorithm: AlgorithmName::SHA1,
            icon: None,
            otp_uri: Some(
                "otpauth://totp/GitHub:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub"
                    .into()
            ),
        };

        let normalized = normalize_payload(&payload, None).expect("payload should parse");
        assert_eq!(normalized.service_name, "GitHub");
        assert_eq!(normalized.account_label.as_deref(), Some("user@example.com"));
        assert_eq!(normalized.secret.as_deref(), Some("JBSWY3DPEHPK3PXP"));
    }

    #[test]
    fn generates_codes_for_supported_algorithms() {
        for algorithm in [
            AlgorithmName::SHA1,
            AlgorithmName::SHA256,
            AlgorithmName::SHA512,
        ] {
            let code = generate_code(
                &test_account(algorithm, 6, 30),
                "JBSWY3DPEHPK3PXP",
            )
            .expect("code should generate");
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|character| character.is_ascii_digit()));
        }
    }

    #[test]
    fn supports_non_default_periods() {
        let seconds = seconds_remaining(45);
        assert!((1..=45).contains(&seconds));
    }

    #[test]
    fn existing_account_serialization_excludes_secret() {
        let account = test_account(AlgorithmName::SHA1, 6, 30);
        let serialized = serde_json::to_string(&account).expect("account should serialize");
        assert!(!serialized.contains("secret"));
    }
}
