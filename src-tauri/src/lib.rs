mod app_error;
mod models;
mod secure_store;
mod totp_logic;

use app_error::{AppError, AppResult};
use arboard::Clipboard;
use models::{AccountEditorContext, AccountMetadata, AccountPayload, CodeView};
use secure_store::{delete_secret, get_secret, initialize_empty_vault, set_secret, vault_exists};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;
use totp_logic::{format_code, normalize_payload, seconds_remaining};

const STORE_FILE: &str = "desktop2fa.json";
const STORE_KEY: &str = "accounts";
const STORAGE_BACKEND_KEY: &str = "secretStorageBackend";
const STORAGE_NOTICE_KEY: &str = "storageResetNotice";
const LOCAL_STORAGE_BACKEND: &str = "local-v1";
const STORAGE_RESET_NOTICE: &str =
    "Desktop2FA switched to local encrypted storage to stop macOS Keychain prompts. Existing accounts were cleared and need to be added again.";

#[derive(Default)]
struct SecretCache(Mutex<HashMap<String, String>>);

#[derive(Default)]
struct EditorContextState(Mutex<AccountEditorContext>);

#[tauri::command]
fn list_accounts(app: AppHandle) -> Result<Vec<AccountMetadata>, String> {
    load_accounts(&app).map_err(AppError::into_command)
}

#[tauri::command]
fn get_active_codes(app: AppHandle, cache: State<'_, SecretCache>) -> Result<Vec<CodeView>, String> {
    load_active_codes(&app, &cache).map_err(AppError::into_command)
}

#[tauri::command]
fn create_account(
    app: AppHandle,
    cache: State<'_, SecretCache>,
    payload: AccountPayload,
) -> Result<AccountMetadata, String> {
    create_account_impl(&app, &cache, payload).map_err(AppError::into_command)
}

#[tauri::command]
fn update_account(
    app: AppHandle,
    cache: State<'_, SecretCache>,
    id: String,
    payload: AccountPayload,
) -> Result<AccountMetadata, String> {
    update_account_impl(&app, &cache, &id, payload).map_err(AppError::into_command)
}

#[tauri::command]
fn delete_account(app: AppHandle, cache: State<'_, SecretCache>, id: String) -> Result<(), String> {
    delete_account_impl(&app, &cache, &id).map_err(AppError::into_command)
}

#[tauri::command]
fn copy_code(app: AppHandle, cache: State<'_, SecretCache>, id: String) -> Result<(), String> {
    copy_code_impl(&app, &cache, &id).map_err(AppError::into_command)
}

#[tauri::command]
fn open_settings_window(app: AppHandle) -> Result<(), String> {
    open_settings_window_impl(&app).map_err(AppError::into_command)
}

#[tauri::command]
fn open_account_editor_window(
    app: AppHandle,
    context: AccountEditorContext,
    editor_context: State<'_, EditorContextState>,
) -> Result<(), String> {
    open_account_editor_window_impl(&app, &editor_context, context).map_err(AppError::into_command)
}

#[tauri::command]
fn get_account_editor_context(
    editor_context: State<'_, EditorContextState>,
) -> Result<AccountEditorContext, String> {
    editor_context
        .0
        .lock()
        .map_err(|_| AppError::Other("Editor context lock was poisoned".into()))
        .map(|context| context.clone())
        .map_err(AppError::into_command)
}

#[tauri::command]
fn hide_main_window(app: AppHandle) -> Result<(), String> {
    hide_main_window_impl(&app).map_err(AppError::into_command)
}

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    show_main_window_impl(&app).map_err(AppError::into_command)
}

#[tauri::command]
fn take_storage_notice(app: AppHandle) -> Result<Option<String>, String> {
    take_storage_notice_impl(&app).map_err(AppError::into_command)
}

fn copy_code_impl(app: &AppHandle, cache: &State<'_, SecretCache>, id: &str) -> AppResult<()> {
    let code = load_active_codes(app, cache)?
        .into_iter()
        .find(|code| code.id == id)
        .ok_or_else(|| AppError::NotFound("account".into()))?;

    let mut clipboard = Clipboard::new().map_err(AppError::Clipboard)?;
    clipboard
        .set_text(code.raw_code)
        .map_err(AppError::Clipboard)?;
    Ok(())
}

fn load_accounts(app: &AppHandle) -> AppResult<Vec<AccountMetadata>> {
    let store = app.store(STORE_FILE)?;
    let Some(value) = store.get(STORE_KEY) else {
        return Ok(Vec::new());
    };

    let mut accounts: Vec<AccountMetadata> = serde_json::from_value(value.clone())?;
    accounts.sort_by_key(|account| account.sort_order);
    Ok(accounts)
}

fn save_accounts(app: &AppHandle, accounts: &[AccountMetadata]) -> AppResult<()> {
    let store = app.store(STORE_FILE)?;
    store.set(STORE_KEY, json!(accounts));
    store.save()?;
    Ok(())
}

fn cache_secret(
    cache: &State<'_, SecretCache>,
    account_id: &str,
    secret: String,
) -> AppResult<()> {
    cache
        .0
        .lock()
        .map_err(|_| AppError::Other("Secret cache lock was poisoned".into()))?
        .insert(account_id.to_string(), secret);
    Ok(())
}

fn remove_cached_secret(cache: &State<'_, SecretCache>, account_id: &str) -> AppResult<()> {
    cache
        .0
        .lock()
        .map_err(|_| AppError::Other("Secret cache lock was poisoned".into()))?
        .remove(account_id);
    Ok(())
}

fn load_active_codes(app: &AppHandle, cache: &State<'_, SecretCache>) -> AppResult<Vec<CodeView>> {
    let accounts = load_accounts(app)?;
    let mut codes = Vec::new();

    for account in accounts {
        let secret = match get_cached_secret_for_app(app, cache, &account.id) {
            Ok(secret) => secret,
            Err(AppError::NotFound(kind)) if kind == "secret" => continue,
            Err(error) => return Err(error),
        };
        let raw_code = totp_logic::generate_code(&account, &secret)?;
        codes.push(CodeView {
            id: account.id,
            service_name: account.service_name,
            account_label: account.account_label,
            formatted_code: format_code(&raw_code),
            raw_code,
            seconds_remaining: seconds_remaining(account.period),
            period: account.period,
            icon: account.icon,
        });
    }

    Ok(codes)
}

fn create_account_impl(
    app: &AppHandle,
    cache: &State<'_, SecretCache>,
    payload: AccountPayload,
) -> AppResult<AccountMetadata> {
    let mut accounts = load_accounts(app)?;
    let normalized = normalize_payload(&payload, None)?;
    let next_sort_order = accounts
        .iter()
        .map(|account| account.sort_order)
        .max()
        .unwrap_or_default()
        + 1;
    let metadata = AccountMetadata {
        id: generate_account_id(),
        service_name: normalized.service_name,
        issuer: normalized.issuer,
        account_label: normalized.account_label,
        digits: normalized.digits,
        period: normalized.period,
        algorithm: normalized.algorithm,
        icon: normalized.icon,
        sort_order: next_sort_order,
    };

    let secret = normalized
        .secret
        .ok_or_else(|| AppError::Validation("Secret is required".into()))?;
    set_secret(app, &metadata.id, &secret)?;
    cache_secret(cache, &metadata.id, secret)?;
    accounts.push(metadata.clone());
    save_accounts(app, &accounts)?;
    let _ = app.emit_to("settings", "accounts-changed", ());

    Ok(metadata)
}

fn generate_account_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    format!("acct-{nanos}")
}

fn update_account_impl(
    app: &AppHandle,
    cache: &State<'_, SecretCache>,
    id: &str,
    payload: AccountPayload,
) -> AppResult<AccountMetadata> {
    let mut accounts = load_accounts(app)?;
    let index = accounts
        .iter()
        .position(|account| account.id == id)
        .ok_or_else(|| AppError::NotFound("account".into()))?;
    let existing = accounts[index].clone();
    let normalized = normalize_payload(&payload, Some(&existing))?;

    if let Some(secret) = normalized.secret {
        set_secret(app, id, &secret)?;
        cache_secret(cache, id, secret)?;
    }

    accounts[index] = AccountMetadata {
        id: existing.id,
        service_name: normalized.service_name,
        issuer: normalized.issuer,
        account_label: normalized.account_label,
        digits: normalized.digits,
        period: normalized.period,
        algorithm: normalized.algorithm,
        icon: normalized.icon,
        sort_order: existing.sort_order,
    };

    save_accounts(app, &accounts)?;
    let _ = app.emit_to("settings", "accounts-changed", ());
    Ok(accounts[index].clone())
}

fn delete_account_impl(app: &AppHandle, cache: &State<'_, SecretCache>, id: &str) -> AppResult<()> {
    let mut accounts = load_accounts(app)?;
    let original_len = accounts.len();
    accounts.retain(|account| account.id != id);
    if accounts.len() == original_len {
        return Err(AppError::NotFound("account".into()));
    }
    for (index, account) in accounts.iter_mut().enumerate() {
        account.sort_order = index as u32;
    }
    save_accounts(app, &accounts)?;
    let _ = remove_cached_secret(cache, id);
    delete_secret(app, id)?;
    let _ = app.emit_to("settings", "accounts-changed", ());
    Ok(())
}

fn get_cached_secret_for_app(
    app: &AppHandle,
    cache: &State<'_, SecretCache>,
    account_id: &str,
) -> AppResult<String> {
    if let Some(secret) = cache
        .0
        .lock()
        .map_err(|_| AppError::Other("Secret cache lock was poisoned".into()))?
        .get(account_id)
        .cloned()
    {
        return Ok(secret);
    }

    let secret = get_secret(app, account_id)?;
    cache
        .0
        .lock()
        .map_err(|_| AppError::Other("Secret cache lock was poisoned".into()))?
        .insert(account_id.to_string(), secret.clone());
    Ok(secret)
}

fn initialize_secret_storage(app: &AppHandle) -> AppResult<()> {
    let store = app.store(STORE_FILE)?;
    let backend = store
        .get(STORAGE_BACKEND_KEY)
        .and_then(|value| value.as_str().map(str::to_string));

    if backend.as_deref() == Some(LOCAL_STORAGE_BACKEND) {
        if !vault_exists(app)? {
            initialize_empty_vault(app)?;
        }
        return Ok(());
    }

    let had_accounts = !load_accounts(app)?.is_empty();
    if had_accounts {
        save_accounts(app, &[])?;
        store.set(STORAGE_NOTICE_KEY, json!(STORAGE_RESET_NOTICE));
    } else {
        store.delete(STORAGE_NOTICE_KEY);
    }

    initialize_empty_vault(app)?;
    store.set(STORAGE_BACKEND_KEY, json!(LOCAL_STORAGE_BACKEND));
    store.save()?;
    Ok(())
}

fn take_storage_notice_impl(app: &AppHandle) -> AppResult<Option<String>> {
    let store = app.store(STORE_FILE)?;
    let notice = store
        .get(STORAGE_NOTICE_KEY)
        .and_then(|value| value.as_str().map(str::to_string));
    if notice.is_some() {
        store.delete(STORAGE_NOTICE_KEY);
        store.save()?;
    }
    Ok(notice)
}

fn apply_window_size(
    window: &tauri::WebviewWindow,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
) -> AppResult<()> {
    window
        .set_min_size(Some(Size::Logical(LogicalSize::new(min_width, min_height))))
        .map_err(|error| AppError::Window(error.to_string()))?;
    window
        .set_size(Size::Logical(LogicalSize::new(width, height)))
        .map_err(|error| AppError::Window(error.to_string()))?;
    window
        .center()
        .map_err(|error| AppError::Window(error.to_string()))?;
    Ok(())
}

fn open_settings_window_impl(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("settings") {
        apply_window_size(&window, 760.0, 240.0, 700.0, 220.0)?;
        window
            .show()
            .map_err(|error| AppError::Window(error.to_string()))?;
        window
            .set_focus()
            .map_err(|error| AppError::Window(error.to_string()))?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, "settings", WebviewUrl::default())
        .title("Desktop2FA Settings")
        .inner_size(760.0, 240.0)
        .min_inner_size(700.0, 220.0)
        .center()
        .build()
        .map_err(|error| AppError::Window(error.to_string()))?;

    apply_window_size(&window, 760.0, 240.0, 700.0, 220.0)?;

    Ok(())
}

fn open_account_editor_window_impl(
    app: &AppHandle,
    editor_context: &State<'_, EditorContextState>,
    context: AccountEditorContext,
) -> AppResult<()> {
    {
        let mut current_context = editor_context
            .0
            .lock()
            .map_err(|_| AppError::Other("Editor context lock was poisoned".into()))?;
        *current_context = context.clone();
    }

    if let Some(window) = app.get_webview_window("account-editor") {
        let _ = app.emit_to("account-editor", "account-editor-context-changed", context);
        apply_window_size(&window, 780.0, 520.0, 760.0, 500.0)?;
        window
            .show()
            .map_err(|error| AppError::Window(error.to_string()))?;
        window
            .set_focus()
            .map_err(|error| AppError::Window(error.to_string()))?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, "account-editor", WebviewUrl::default())
        .title("Desktop2FA Account")
        .inner_size(780.0, 520.0)
        .min_inner_size(760.0, 500.0)
        .center()
        .build()
        .map_err(|error| AppError::Window(error.to_string()))?;

    apply_window_size(&window, 780.0, 520.0, 760.0, 500.0)?;

    let _ = app.emit_to("account-editor", "account-editor-context-changed", context);
    window
        .set_focus()
        .map_err(|error| AppError::Window(error.to_string()))?;

    Ok(())
}

fn show_main_window_impl(app: &AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Window("Main window is unavailable".into()))?;
    window
        .show()
        .map_err(|error| AppError::Window(error.to_string()))?;
    window
        .unminimize()
        .map_err(|error| AppError::Window(error.to_string()))?;
    window
        .set_focus()
        .map_err(|error| AppError::Window(error.to_string()))?;
    Ok(())
}

fn hide_main_window_impl(app: &AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Window("Main window is unavailable".into()))?;
    window
        .hide()
        .map_err(|error| AppError::Window(error.to_string()))?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> AppResult<()> {
    let menu = MenuBuilder::new(app)
        .text("show", "Show App")
        .text("settings", "Settings")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let mut tray_builder = TrayIconBuilder::new().menu(&menu);
    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder
        .on_tray_icon_event({
            let app = app.clone();
            move |_tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let _ = show_main_window_impl(&app);
                }
            }
        })
        .on_menu_event({
            let app = app.clone();
            move |_tray, event| match event.id.as_ref() {
                "show" => {
                    let _ = show_main_window_impl(&app);
                }
                "settings" => {
                    let _ = open_settings_window_impl(&app);
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)
        .map_err(|error| AppError::Window(error.to_string()))?;

    Ok(())
}

fn build_app_menu(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let app_menu = SubmenuBuilder::new(app, "Desktop2FA")
        .text("settings", "Settings...")
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .separator()
        .text("show", "Show Desktop2FA")
        .close_window()
        .build()?;

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&window_menu)
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SecretCache::default())
        .manage(EditorContextState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .menu(build_app_menu)
        .on_menu_event(|app, event| match event.id().0.as_ref() {
            "settings" => {
                let _ = open_settings_window_impl(app);
            }
            "show" => {
                let _ = show_main_window_impl(app);
            }
            _ => {}
        })
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;

            initialize_secret_storage(&app.handle())?;
            build_tray(&app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_accounts,
            get_active_codes,
            create_account,
            update_account,
            delete_account,
            copy_code,
            open_settings_window,
            open_account_editor_window,
            get_account_editor_context,
            hide_main_window,
            show_main_window,
            take_storage_notice
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
