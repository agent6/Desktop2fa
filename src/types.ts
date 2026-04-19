export type TotpAlgorithm = "SHA1" | "SHA256" | "SHA512";

export type TotpAccountSummary = {
  id: string;
  serviceName: string;
  issuer?: string;
  accountLabel?: string;
  digits: number;
  period: number;
  algorithm: TotpAlgorithm;
  icon?: string;
  sortOrder: number;
};

export type TotpCodeView = {
  id: string;
  serviceName: string;
  accountLabel?: string;
  formattedCode: string;
  rawCode: string;
  secondsRemaining: number;
  period: number;
  icon?: string;
};

export type AccountPayload = {
  serviceName: string;
  issuer?: string;
  accountLabel?: string;
  secret?: string;
  digits: number;
  period: number;
  algorithm: TotpAlgorithm;
  icon?: string;
  otpUri?: string;
};

export type AccountEditorMode = "create" | "edit";

export type AccountEditorContext = {
  mode: AccountEditorMode;
  accountId?: string;
};
