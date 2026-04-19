import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { WidgetShell } from "./components/WidgetShell";
import { TotpRow } from "./components/TotpRow";
import { AccountEditorWindow } from "./features/settings/AccountEditorWindow";
import { SettingsListWindow } from "./features/settings/SettingsListWindow";
import { api } from "./lib/api";
import type { TotpCodeView } from "./types";

function EmptyState() {
  return (
    <div className="px-8 py-16 text-center">
      <div className="mx-auto max-w-xs">
        <div className="text-2xl font-semibold text-slate-700">No accounts yet</div>
        <p className="mt-3 text-sm leading-6 text-slate-400">
          Open <span className="font-medium text-slate-500">Desktop2FA &gt; Settings…</span> from the
          menu bar to add your first account.
        </p>
      </div>
    </div>
  );
}

function WidgetApp() {
  const [codes, setCodes] = useState<TotpCodeView[]>([]);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const panelRef = useRef<HTMLElement | null>(null);
  const shouldScrollList = codes.length > 5;

  async function refreshCodes() {
    try {
      const activeCodes = await api.getActiveCodes();
      setCodes(activeCodes);
      setError("");
    } catch (refreshError) {
      setError(String(refreshError));
    }
  }

  useEffect(() => {
    void refreshCodes();
    void api.takeStorageNotice().then(setNotice).catch(() => undefined);
    const interval = window.setInterval(() => {
      void refreshCodes();
    }, 1000);

    return () => window.clearInterval(interval);
  }, []);

  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) {
      return;
    }

    let cancelled = false;

    const resizeWindow = async () => {
      const currentWindow = getCurrentWindow();
      const inner = await currentWindow.innerSize();
      const outer = await currentWindow.outerSize();
      const chromeHeight = outer.height - inner.height;
      const measuredHeight = Math.ceil(panel.getBoundingClientRect().height);
      const targetHeight = Math.max(180, measuredHeight + chromeHeight);
      const targetWidth = Math.max(500, inner.width);

      if (!cancelled) {
        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setMinSize(new LogicalSize(500, targetHeight));
      }
    };

    const observer = new ResizeObserver(() => {
      void resizeWindow();
    });
    observer.observe(panel);

    const frame = window.requestAnimationFrame(() => {
      void resizeWindow();
    });

    return () => {
      cancelled = true;
      observer.disconnect();
      window.cancelAnimationFrame(frame);
    };
  }, [codes.length, error, notice]);

  async function handleCopy(id: string) {
    try {
      await api.copyCode(id);
      setCopiedId(id);
      window.setTimeout(() => {
        setCopiedId((current) => (current === id ? null : current));
      }, 1200);
    } catch (copyError) {
      setError(String(copyError));
    }
  }

  return (
    <WidgetShell>
      <section ref={panelRef} className="mini-app-panel">
        {notice ? (
          <div className="mx-4 mt-4 border border-amber-100 bg-amber-50 px-4 py-3 text-sm text-amber-700">
            {notice}
          </div>
        ) : null}
        {error ? (
          <div className="mx-4 mt-4 border border-amber-100 bg-amber-50 px-4 py-3 text-sm text-amber-700">
            {error}
          </div>
        ) : null}
        {codes.length === 0 ? (
          <EmptyState />
        ) : (
          <div
            className={`mini-app-list border-y border-slate-200 ${
              shouldScrollList ? "overflow-y-auto" : "overflow-hidden"
            }`}
          >
            {codes.map((code) => (
              <TotpRow key={code.id} code={code} isCopied={copiedId === code.id} onCopy={handleCopy} />
            ))}
          </div>
        )}
      </section>
    </WidgetShell>
  );
}

export default function App() {
  const label = useMemo(() => getCurrentWebviewWindow().label, []);

  useEffect(() => {
    document.body.dataset.window = label;
    document.documentElement.dataset.window = label;
    return () => {
      delete document.body.dataset.window;
      delete document.documentElement.dataset.window;
    };
  }, [label]);

  if (label === "settings") {
    return <SettingsListWindow />;
  }

  if (label === "account-editor") {
    return <AccountEditorWindow />;
  }

  return <WidgetApp />;
}
