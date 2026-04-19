import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AccountEditorWindow } from "./AccountEditorWindow";

const listAccounts = vi.fn();
const createAccount = vi.fn();
const updateAccount = vi.fn();
const getAccountEditorContext = vi.fn();
const openSettingsWindow = vi.fn();
const closeWindow = vi.fn();
const listen = vi.fn().mockResolvedValue(() => {});

class ResizeObserverMock {
  observe() {}
  disconnect() {}
}

vi.stubGlobal("ResizeObserver", ResizeObserverMock);

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listen(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
  getCurrentWindow: () => ({
    innerSize: () => Promise.resolve({ width: 640, height: 600 }),
    outerSize: () => Promise.resolve({ width: 656, height: 638 }),
    setSize: vi.fn().mockResolvedValue(undefined),
    setMinSize: vi.fn().mockResolvedValue(undefined),
    close: () => closeWindow(),
  }),
}));

vi.mock("../../lib/api", () => ({
  api: {
    listAccounts: () => listAccounts(),
    createAccount: (...args: unknown[]) => createAccount(...args),
    updateAccount: (...args: unknown[]) => updateAccount(...args),
    getAccountEditorContext: () => getAccountEditorContext(),
    openSettingsWindow: () => openSettingsWindow(),
  },
}));

describe("AccountEditorWindow", () => {
  beforeEach(() => {
    listAccounts.mockResolvedValue([]);
    createAccount.mockResolvedValue(undefined);
    updateAccount.mockResolvedValue(undefined);
    getAccountEditorContext.mockResolvedValue({ mode: "create" });
    openSettingsWindow.mockResolvedValue(undefined);
    closeWindow.mockResolvedValue(undefined);
  });

  it("saves a manual account", async () => {
    const user = userEvent.setup();
    render(<AccountEditorWindow />);

    await waitFor(() => expect(getAccountEditorContext).toHaveBeenCalled());
    await user.type(screen.getByLabelText(/service name/i), "GitHub");
    await user.type(screen.getByLabelText(/^secret/i), "JBSWY3DPEHPK3PXP");
    await user.click(screen.getByRole("button", { name: /add account/i }));

    await waitFor(() =>
      expect(createAccount).toHaveBeenCalledWith(
        expect.objectContaining({
          serviceName: "GitHub",
          secret: "JBSWY3DPEHPK3PXP",
          digits: 6,
          period: 30,
          algorithm: "SHA1",
        }),
      ),
    );
  });
});
