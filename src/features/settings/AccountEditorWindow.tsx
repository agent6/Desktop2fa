import { listen } from "@tauri-apps/api/event";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { api } from "../../lib/api";
import type {
  AccountEditorContext,
  AccountPayload,
  TotpAccountSummary,
  TotpAlgorithm,
} from "../../types";

const DEFAULT_FORM: AccountPayload = {
  serviceName: "",
  issuer: "",
  accountLabel: "",
  secret: "",
  digits: 6,
  period: 30,
  algorithm: "SHA1",
  otpUri: "",
};

function mapAccountToForm(account: TotpAccountSummary): AccountPayload {
  return {
    serviceName: account.serviceName,
    issuer: account.issuer ?? "",
    accountLabel: account.accountLabel ?? "",
    secret: "",
    digits: account.digits,
    period: account.period,
    algorithm: account.algorithm,
    otpUri: "",
  };
}

function normalizeFieldValue(value: string) {
  return value.trim();
}

export function AccountEditorWindow() {
  const [context, setContext] = useState<AccountEditorContext>({ mode: "create" });
  const [form, setForm] = useState<AccountPayload>(DEFAULT_FORM);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLElement | null>(null);

  const isEditing = context.mode === "edit" && Boolean(context.accountId);

  const title = useMemo(
    () => (isEditing ? "Update TOTP account" : "Add a TOTP account"),
    [isEditing],
  );

  async function loadContext(nextContext?: AccountEditorContext) {
    setLoading(true);
    setError("");

    try {
      const currentContext = nextContext ?? (await api.getAccountEditorContext());
      setContext(currentContext);

      if (currentContext.mode === "edit" && currentContext.accountId) {
        const accounts = await api.listAccounts();
        const account = accounts.find((item) => item.id === currentContext.accountId);
        if (!account) {
          throw new Error("Could not find account to edit.");
        }
        setForm(mapAccountToForm(account));
        setAdvancedOpen(
          Boolean(account.accountLabel || account.issuer) ||
            account.algorithm !== "SHA1" ||
            account.digits !== 6 ||
            account.period !== 30,
        );
      } else {
        setForm(DEFAULT_FORM);
        setAdvancedOpen(false);
      }
    } catch (loadError) {
      setError(String(loadError));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadContext();

    let unlisten: (() => void) | undefined;
    void listen<AccountEditorContext>("account-editor-context-changed", (event) => {
      void loadContext(event.payload);
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  useLayoutEffect(() => {
    const panel = panelRef.current;
    const content = contentRef.current;
    if (!panel || !content) {
      return;
    }

    let cancelled = false;

    const resizeWindow = async () => {
      const currentWindow = getCurrentWindow();
      const inner = await currentWindow.innerSize();
      const outer = await currentWindow.outerSize();
      const chromeHeight = outer.height - inner.height;
      const chromeWidth = outer.width - inner.width;
      const measuredWidth = Math.ceil(Math.max(content.scrollWidth, content.getBoundingClientRect().width));
      const measuredHeight = Math.ceil(
        Math.max(panel.scrollHeight, content.scrollHeight, content.getBoundingClientRect().height + 56),
      );
      const targetWidth = Math.max(760, Math.min(820, measuredWidth + chromeWidth + 24));
      const targetHeight = Math.max(500, measuredHeight + chromeHeight);

      if (!cancelled) {
        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setMinSize(new LogicalSize(760, 500));
      }
    };

    const observer = new ResizeObserver(() => {
      void resizeWindow();
    });
    observer.observe(panel);
    observer.observe(content);

    const frame = window.requestAnimationFrame(() => {
      void resizeWindow();
    });

    return () => {
      cancelled = true;
      observer.disconnect();
      window.cancelAnimationFrame(frame);
    };
  }, [context.mode, error, loading, advancedOpen]);

  function updateField<K extends keyof AccountPayload>(key: K, value: AccountPayload[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function closeWindow() {
    await api.openSettingsWindow();
    await getCurrentWindow().close();
  }

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    setError("");
    setSaving(true);

    const payload: AccountPayload = {
      serviceName: normalizeFieldValue(form.serviceName),
      issuer: normalizeFieldValue(form.issuer ?? ""),
      accountLabel: normalizeFieldValue(form.accountLabel ?? ""),
      secret: normalizeFieldValue(form.secret ?? ""),
      digits: form.digits,
      period: form.period,
      algorithm: form.algorithm,
      otpUri: normalizeFieldValue(form.otpUri ?? ""),
    };

    try {
      if (context.mode === "edit" && context.accountId) {
        await api.updateAccount(context.accountId, payload);
      } else {
        await api.createAccount(payload);
      }
      await closeWindow();
    } catch (submitError) {
      setError(String(submitError));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div ref={panelRef} className="account-editor-app bg-slate-50 p-4 text-slate-700">
      <section
        ref={contentRef}
        className="account-editor-shell mx-auto flex h-[calc(100vh-2rem)] w-[720px] max-w-none flex-col border border-slate-200 bg-white"
      >
        <header className="border-b border-slate-200 px-5 py-4">
          <div className="text-[0.7rem] font-semibold uppercase tracking-[0.22em] text-slate-400">
            {isEditing ? "Edit Account" : "New Account"}
          </div>
          <h1 className="mt-1 text-2xl font-semibold text-slate-800">{title}</h1>
          <p className="mt-1 text-sm text-slate-500">
            Encrypted local storage with a compact desktop form.
          </p>
        </header>

        {error ? (
          <div className="mx-5 mt-4 border border-rose-100 bg-rose-50 px-4 py-3 text-sm text-rose-700">
            {error}
          </div>
        ) : null}

        <form className="flex min-h-0 flex-1 flex-col" onSubmit={(event) => void handleSubmit(event)}>
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-5">
            <Field label="Service name" required>
              <input
                aria-label="Service name"
                value={form.serviceName}
                onChange={(event) => updateField("serviceName", event.currentTarget.value)}
                className="field"
                placeholder="GitHub"
                required={!form.otpUri}
                disabled={loading || saving}
              />
            </Field>

            <Field
              label={isEditing ? "Secret (leave blank to keep)" : "Secret"}
              required={!isEditing && !form.otpUri}
            >
              <input
                aria-label={isEditing ? "Secret (leave blank to keep)" : "Secret"}
                value={form.secret}
                onChange={(event) => updateField("secret", event.currentTarget.value)}
                className="field mono-code"
                placeholder="BASE32SECRET"
                required={!isEditing && !form.otpUri}
                disabled={loading || saving}
              />
            </Field>

            <details
              className="border border-slate-200 bg-slate-50"
              open={advancedOpen}
              onToggle={(event) => setAdvancedOpen((event.currentTarget as HTMLDetailsElement).open)}
            >
              <summary className="cursor-pointer select-none px-4 py-3 text-sm font-medium text-slate-700">
                Advanced settings
              </summary>
              <div className="space-y-4 border-t border-slate-200 bg-white p-4">
                <label className="block">
                  <span className="mb-1.5 block text-sm font-medium text-slate-600">OTP URI</span>
                  <textarea
                    value={form.otpUri}
                    onChange={(event) => updateField("otpUri", event.currentTarget.value)}
                    placeholder="otpauth://totp/GitHub:user@example.com?secret=BASE32SECRET&issuer=GitHub"
                    className="field min-h-[84px] resize-none"
                    disabled={loading || saving}
                  />
                  <span className="mt-1 block text-xs text-slate-400">
                    URI import overrides manual secret fields.
                  </span>
                </label>

                <div className="grid gap-4 md:grid-cols-2">
                  <Field label="Account label">
                    <input
                      aria-label="Account label"
                      value={form.accountLabel}
                      onChange={(event) => updateField("accountLabel", event.currentTarget.value)}
                      className="field"
                      placeholder="name@company.com"
                      disabled={loading || saving}
                    />
                  </Field>
                  <Field label="Issuer">
                    <input
                      aria-label="Issuer"
                      value={form.issuer}
                      onChange={(event) => updateField("issuer", event.currentTarget.value)}
                      className="field"
                      placeholder="GitHub"
                      disabled={loading || saving}
                    />
                  </Field>
                  <Field label="Algorithm">
                    <select
                      aria-label="Algorithm"
                      value={form.algorithm}
                      onChange={(event) => updateField("algorithm", event.currentTarget.value as TotpAlgorithm)}
                      className="field"
                      disabled={loading || saving}
                    >
                      <option value="SHA1">SHA1</option>
                      <option value="SHA256">SHA256</option>
                      <option value="SHA512">SHA512</option>
                    </select>
                  </Field>
                  <Field label="Digits">
                    <input
                      aria-label="Digits"
                      type="number"
                      min={6}
                      max={8}
                      value={form.digits}
                      onChange={(event) => updateField("digits", Number(event.currentTarget.value))}
                      className="field"
                      disabled={loading || saving}
                    />
                  </Field>
                  <Field label="Period (seconds)">
                    <input
                      aria-label="Period (seconds)"
                      type="number"
                      min={15}
                      max={120}
                      step={5}
                      value={form.period}
                      onChange={(event) => updateField("period", Number(event.currentTarget.value))}
                      className="field"
                      disabled={loading || saving}
                    />
                  </Field>
                </div>

                <div className="border border-slate-200 bg-slate-50 px-3 py-3 text-xs leading-5 text-slate-500">
                  Leave secret blank while editing to keep the current keychain entry.
                </div>
              </div>
            </details>
          </div>

          <div className="flex items-center gap-3 border-t border-slate-200 px-5 py-4">
            <button
              type="submit"
              disabled={saving || loading}
              className="bg-slate-800 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:bg-slate-400"
            >
              {saving ? "Saving..." : isEditing ? "Save changes" : "Add account"}
            </button>
            <button
              type="button"
              onClick={() => void closeWindow()}
              disabled={saving}
              className="border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 transition hover:border-slate-400 disabled:cursor-not-allowed disabled:text-slate-400"
            >
              Cancel
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}

function Field({
  children,
  label,
  required = false,
}: {
  children: React.ReactNode;
  label: string;
  required?: boolean;
}) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm font-medium text-slate-600">
        {label}
        {required ? <span className="ml-1 text-rose-500">*</span> : null}
      </span>
      {children}
    </label>
  );
}
