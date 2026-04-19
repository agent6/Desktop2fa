use aes_gcm_siv::aead::{Aead, KeyInit};
use aes_gcm_siv::{Aes256GcmSiv, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::app_error::{AppError, AppResult};

const VAULT_VERSION: u32 = 1;
const VAULT_FILE_NAME: &str = "secrets.vault.json";
const KEY_FILE_NAME: &str = "secrets.key";
const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone)]
struct StoragePaths {
    root_dir: PathBuf,
    vault_file: PathBuf,
    key_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct VaultPayload {
    secrets: HashMap<String, String>,
}

pub fn vault_exists(app: &AppHandle) -> AppResult<bool> {
    Ok(resolve_storage_paths(app)?.vault_file.exists())
}

pub fn initialize_empty_vault(app: &AppHandle) -> AppResult<()> {
    let paths = resolve_storage_paths(app)?;
    ensure_storage_root(&paths.root_dir)?;
    ensure_key(&paths)?;
    if !paths.vault_file.exists() {
      save_secret_map_to_paths(&paths, &HashMap::new())?;
    }
    Ok(())
}

pub fn set_secret(app: &AppHandle, account_id: &str, secret: &str) -> AppResult<()> {
    let paths = resolve_storage_paths(app)?;
    let mut secrets = load_secret_map_from_paths(&paths)?;
    secrets.insert(account_id.to_string(), secret.to_string());
    save_secret_map_to_paths(&paths, &secrets)
}

pub fn get_secret(app: &AppHandle, account_id: &str) -> AppResult<String> {
    let paths = resolve_storage_paths(app)?;
    let secrets = load_secret_map_from_paths(&paths)?;
    secrets
        .get(account_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("secret".into()))
}

pub fn delete_secret(app: &AppHandle, account_id: &str) -> AppResult<()> {
    let paths = resolve_storage_paths(app)?;
    let mut secrets = load_secret_map_from_paths(&paths)?;
    secrets.remove(account_id);
    save_secret_map_to_paths(&paths, &secrets)
}

fn resolve_storage_paths(app: &AppHandle) -> AppResult<StoragePaths> {
    let root_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| AppError::Persistence(error.to_string()))?
        .join("vault");
    Ok(StoragePaths {
        vault_file: root_dir.join(VAULT_FILE_NAME),
        key_file: root_dir.join(KEY_FILE_NAME),
        root_dir,
    })
}

fn load_secret_map_from_paths(paths: &StoragePaths) -> AppResult<HashMap<String, String>> {
    ensure_storage_root(&paths.root_dir)?;
    let key = ensure_key(paths)?;
    if !paths.vault_file.exists() {
        save_secret_map_to_paths(paths, &HashMap::new())?;
    }

    let raw = fs::read(&paths.vault_file)?;
    let vault: VaultFile = serde_json::from_slice(&raw)?;
    if vault.version != VAULT_VERSION {
        return Err(AppError::SecureStore("Secret vault version is unsupported".into()));
    }
    if vault.nonce.len() != NONCE_SIZE {
        return Err(AppError::SecureStore("Secret vault nonce is invalid".into()));
    }

    let cipher = Aes256GcmSiv::new_from_slice(&key)
        .map_err(|error| AppError::SecureStore(error.to_string()))?;
    let decrypted = cipher
        .decrypt(Nonce::from_slice(&vault.nonce), vault.ciphertext.as_ref())
        .map_err(|_| AppError::SecureStore("Secret vault could not be decrypted".into()))?;
    let payload: VaultPayload = serde_json::from_slice(&decrypted)?;
    Ok(payload.secrets)
}

fn save_secret_map_to_paths(paths: &StoragePaths, secrets: &HashMap<String, String>) -> AppResult<()> {
    ensure_storage_root(&paths.root_dir)?;
    let key = ensure_key(paths)?;
    let cipher = Aes256GcmSiv::new_from_slice(&key)
        .map_err(|error| AppError::SecureStore(error.to_string()))?;

    let mut nonce = [0_u8; NONCE_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut nonce);

    let payload = serde_json::to_vec(&VaultPayload {
        secrets: secrets.clone(),
    })?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), payload.as_ref())
        .map_err(|_| AppError::SecureStore("Secret vault could not be encrypted".into()))?;
    let encoded = serde_json::to_vec_pretty(&VaultFile {
        version: VAULT_VERSION,
        nonce: nonce.to_vec(),
        ciphertext,
    })?;

    let temp_path = paths.vault_file.with_extension("json.tmp");
    fs::write(&temp_path, encoded)?;
    set_file_permissions(&temp_path)?;
    fs::rename(temp_path, &paths.vault_file)?;
    set_file_permissions(&paths.vault_file)?;
    Ok(())
}

fn ensure_key(paths: &StoragePaths) -> AppResult<[u8; KEY_SIZE]> {
    if paths.key_file.exists() {
        let raw = fs::read(&paths.key_file)?;
        return raw
            .try_into()
            .map_err(|_| AppError::SecureStore("Secret vault key is invalid".into()));
    }

    let mut key = [0_u8; KEY_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut key);
    fs::write(&paths.key_file, key)?;
    set_file_permissions(&paths.key_file)?;
    Ok(key)
}

fn ensure_storage_root(root_dir: &Path) -> AppResult<()> {
    fs::create_dir_all(root_dir)?;
    set_directory_permissions(root_dir)?;
    Ok(())
}

#[cfg(unix)]
fn set_directory_permissions(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_directory_permissions(_path: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_file_permissions(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_file_permissions(_path: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_paths(name: &str) -> StoragePaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let root_dir = std::env::temp_dir().join(format!("desktop2fa-{name}-{suffix}"));
        StoragePaths {
            vault_file: root_dir.join(VAULT_FILE_NAME),
            key_file: root_dir.join(KEY_FILE_NAME),
            root_dir,
        }
    }

    #[test]
    fn vault_round_trip() {
        let paths = temp_paths("round-trip");
        let mut input = HashMap::new();
        input.insert("acct-1".into(), "JBSWY3DPEHPK3PXP".into());

        save_secret_map_to_paths(&paths, &input).expect("vault should save");
        let loaded = load_secret_map_from_paths(&paths).expect("vault should load");

        assert_eq!(loaded, input);
        let _ = fs::remove_dir_all(paths.root_dir);
    }

    #[test]
    fn wrong_key_returns_error() {
        let paths = temp_paths("wrong-key");
        let mut input = HashMap::new();
        input.insert("acct-1".into(), "SECRET".into());
        save_secret_map_to_paths(&paths, &input).expect("vault should save");

        fs::write(&paths.key_file, [7_u8; KEY_SIZE]).expect("key should overwrite");
        let error = load_secret_map_from_paths(&paths).expect_err("vault should not decrypt");

        assert!(matches!(error, AppError::SecureStore(_)));
        let _ = fs::remove_dir_all(paths.root_dir);
    }

    #[test]
    fn corrupt_vault_returns_error() {
        let paths = temp_paths("corrupt-vault");
        ensure_storage_root(&paths.root_dir).expect("root should exist");
        let _ = ensure_key(&paths).expect("key should exist");
        fs::write(&paths.vault_file, b"not-json").expect("vault should write");

        let error = load_secret_map_from_paths(&paths).expect_err("vault should fail");
        assert!(matches!(error, AppError::Persistence(_)));
        let _ = fs::remove_dir_all(paths.root_dir);
    }

    #[test]
    fn secret_crud_persists() {
        let paths = temp_paths("crud");
        save_secret_map_to_paths(&paths, &HashMap::new()).expect("empty vault should save");

        let mut secrets = load_secret_map_from_paths(&paths).expect("vault should load");
        secrets.insert("acct-1".into(), "ONE".into());
        save_secret_map_to_paths(&paths, &secrets).expect("vault should update");

        let mut loaded = load_secret_map_from_paths(&paths).expect("vault should load");
        assert_eq!(loaded.get("acct-1").map(String::as_str), Some("ONE"));

        loaded.remove("acct-1");
        save_secret_map_to_paths(&paths, &loaded).expect("vault should delete");
        let final_state = load_secret_map_from_paths(&paths).expect("vault should reload");
        assert!(final_state.is_empty());
        let _ = fs::remove_dir_all(paths.root_dir);
    }
}
