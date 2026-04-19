import { invoke } from "@tauri-apps/api/core";
import type {
  AccountEditorContext,
  AccountPayload,
  TotpAccountSummary,
  TotpCodeView,
} from "../types";

export const api = {
  listAccounts: () => invoke<TotpAccountSummary[]>("list_accounts"),
  getActiveCodes: () => invoke<TotpCodeView[]>("get_active_codes"),
  createAccount: (payload: AccountPayload) =>
    invoke<TotpAccountSummary>("create_account", { payload }),
  updateAccount: (id: string, payload: AccountPayload) =>
    invoke<TotpAccountSummary>("update_account", { id, payload }),
  deleteAccount: (id: string) => invoke<void>("delete_account", { id }),
  copyCode: (id: string) => invoke<void>("copy_code", { id }),
  openSettingsWindow: () => invoke<void>("open_settings_window"),
  openAccountEditorWindow: (context: AccountEditorContext) =>
    invoke<void>("open_account_editor_window", { context }),
  getAccountEditorContext: () => invoke<AccountEditorContext>("get_account_editor_context"),
  takeStorageNotice: () => invoke<string | null>("take_storage_notice"),
  hideMainWindow: () => invoke<void>("hide_main_window"),
  showMainWindow: () => invoke<void>("show_main_window"),
};
