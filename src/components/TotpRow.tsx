import type { TotpCodeView } from "../types";

const DEFAULT_ACCOUNT_ICON = "/account-icons/icon-01.png";

type TotpRowProps = {
  code: TotpCodeView;
  isCopied: boolean;
  onCopy: (id: string) => void;
};

function CopyIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" className="h-5 w-5">
      <rect x="9" y="9" width="10" height="10" rx="2" />
      <path d="M6 15H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="h-5 w-5">
      <path d="m5 12 4.2 4.2L19 6.5" />
    </svg>
  );
}

function ServiceAvatar({ icon }: { icon?: string }) {
  return (
    <img
      src={icon && /^(https?:|data:image|\/)/.test(icon) ? icon : DEFAULT_ACCOUNT_ICON}
      alt=""
      className="h-12 w-12 rounded-2xl border border-slate-200/70 bg-white object-cover"
    />
  );
}

export function TotpRow({ code, isCopied, onCopy }: TotpRowProps) {
  const progress = ((code.period - code.secondsRemaining) / code.period) * 100;

  return (
    <div className="border-b border-slate-200/70 last:border-b-0">
      <div className="flex min-h-[104px] items-center gap-4 px-5 py-4">
        <ServiceAvatar icon={code.icon} />
        <div className="min-w-0 flex-1">
          <div className="truncate text-[1.15rem] font-semibold text-slate-700">
            {code.serviceName}
          </div>
          <div className="truncate text-sm text-slate-400">
            {code.accountLabel ?? `${code.secondsRemaining}s remaining`}
          </div>
        </div>
        <div className="flex items-center gap-3">
          <div className="text-right">
            <div className="mono-code text-[1.9rem] font-semibold tracking-[0.16em] text-slate-600">
              {code.formattedCode}
            </div>
            <div className="mt-2 h-1.5 w-24 overflow-hidden rounded-full bg-slate-200/80">
              <div
                className="h-full rounded-full bg-[linear-gradient(90deg,#83a6d7,#6386bf)] transition-[width] duration-700"
                style={{ width: `${Math.max(6, progress)}%` }}
              />
            </div>
          </div>
          <button
            type="button"
            aria-label={`Copy ${code.serviceName} code`}
            onClick={() => onCopy(code.id)}
            data-no-drag
            className={`flex h-12 w-12 items-center justify-center border transition ${
              isCopied
                ? "border-emerald-200 bg-emerald-50 text-emerald-600"
                : "border-slate-200/80 bg-white/70 text-slate-400 hover:border-slate-300 hover:text-slate-600"
            } focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-slate-400`}
          >
            {isCopied ? <CheckIcon /> : <CopyIcon />}
          </button>
        </div>
      </div>
    </div>
  );
}
