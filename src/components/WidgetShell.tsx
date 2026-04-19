import type { ReactNode } from "react";

type WidgetShellProps = {
  children: ReactNode;
};

export function WidgetShell({ children }: WidgetShellProps) {
  return <main className="mini-app-shell bg-slate-50 text-slate-700">{children}</main>;
}
