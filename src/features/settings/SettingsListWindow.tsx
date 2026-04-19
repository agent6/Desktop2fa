import { listen } from "@tauri-apps/api/event";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { api } from "../../lib/api";
import type { AccountEditorContext, TotpAccountSummary } from "../../types";

type SettingsListWindowProps = {
  initialAccounts?: TotpAccountSummary[];
};

export function SettingsListWindow({ initialAccounts = [] }: SettingsListWindowProps) {
  const [accounts, setAccounts] = useState<TotpAccountSummary[]>(initialAccounts);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLElement | null>(null);

  async function reloadAccounts() {
    try {
      const nextAccounts = await api.listAccounts();
      setAccounts(nextAccounts);
      setError("");
    } catch (reloadError) {
      setError(String(reloadError));
    }
  }

  useEffect(() => {
    void reloadAccounts();
    void api.takeStorageNotice().then(setNotice).catch(() => undefined);

    let unlisten: (() => void) | undefined;
    void listen("accounts-changed", () => {
      void reloadAccounts();
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
        Math.max(panel.scrollHeight, content.scrollHeight, content.getBoundingClientRect().height + 32),
      );
      const targetWidth = Math.max(620, Math.min(760, measuredWidth + chromeWidth + 32));
      const targetHeight = Math.max(220, Math.min(900, measuredHeight + chromeHeight));

      if (!cancelled) {
        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setMinSize(new LogicalSize(620, 220));
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
  }, [accounts.length, error, notice]);

  async function openEditor(context: AccountEditorContext) {
    try {
      setError("");
      await api.openAccountEditorWindow(context);
    } catch (openError) {
      setError(String(openError));
    }
  }

  async function handleDelete(id: string) {
    try {
      setError("");
      await api.deleteAccount(id);
      await reloadAccounts();
    } catch (deleteError) {
      setError(String(deleteError));
    }
  }

  return (
    <div ref={panelRef} className="settings-list-app bg-slate-50 p-4 text-slate-700">
      <section ref={contentRef} className="settings-list-shell border border-slate-200 bg-white">
        <header className="flex items-center justify-between border-b border-slate-200 px-4 py-3">
          <div>
            <div className="text-[0.7rem] font-semibold uppercase tracking-[0.22em] text-slate-400">
              Accounts
            </div>
            <div className="mt-1 text-xl font-semibold text-slate-700">{accounts.length}</div>
          </div>
          <button
            type="button"
            onClick={() => void openEditor({ mode: "create" })}
            className="border border-slate-300 bg-white px-3 py-1.5 text-sm font-medium text-slate-700 transition hover:border-slate-400"
          >
            New
          </button>
        </header>

        {notice ? (
          <div className="mx-4 mt-4 border border-amber-100 bg-amber-50 px-4 py-3 text-sm text-amber-700">
            {notice}
          </div>
        ) : null}

        {error ? (
          <div className="mx-4 mt-4 border border-rose-100 bg-rose-50 px-4 py-3 text-sm text-rose-700">
            {error}
          </div>
        ) : null}

        <div className="settings-list p-3">
          {accounts.length === 0 ? (
            <div className="border border-dashed border-slate-200 bg-slate-50 px-4 py-6 text-sm text-slate-400">
              No accounts saved yet.
            </div>
          ) : null}
          <div className="space-y-2">
            {accounts.map((account) => (
              <div key={account.id} className="border border-slate-200 bg-white px-3 py-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="truncate text-base font-semibold text-slate-700">
                      {account.serviceName}
                    </div>
                    <div className="mt-1 truncate text-sm text-slate-400">
                      {account.accountLabel || account.issuer || `${account.period}s · ${account.algorithm}`}
                    </div>
                  </div>
                  <div className="flex shrink-0 gap-2">
                    <button
                      type="button"
                      onClick={() => void openEditor({ mode: "edit", accountId: account.id })}
                      className="border border-slate-200 bg-white px-2.5 py-1 text-xs font-medium text-slate-500 transition hover:border-slate-300 hover:text-slate-700"
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleDelete(account.id)}
                      className="border border-rose-100 bg-rose-50 px-2.5 py-1 text-xs font-medium text-rose-600 transition hover:border-rose-200 hover:bg-rose-100"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
