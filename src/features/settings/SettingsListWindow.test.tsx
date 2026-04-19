import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsListWindow } from "./SettingsListWindow";

const listAccounts = vi.fn();
const deleteAccount = vi.fn();
const openAccountEditorWindow = vi.fn();
const takeStorageNotice = vi.fn();
const listen = vi.fn().mockResolvedValue(() => {});
const setSize = vi.fn().mockResolvedValue(undefined);
const setMinSize = vi.fn().mockResolvedValue(undefined);

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
    innerSize: () => Promise.resolve({ width: 480, height: 240 }),
    outerSize: () => Promise.resolve({ width: 496, height: 278 }),
    setSize: (...args: unknown[]) => setSize(...args),
    setMinSize: (...args: unknown[]) => setMinSize(...args),
  }),
}));

vi.mock("../../lib/api", () => ({
  api: {
    listAccounts: () => listAccounts(),
    deleteAccount: (...args: unknown[]) => deleteAccount(...args),
    openAccountEditorWindow: (...args: unknown[]) => openAccountEditorWindow(...args),
    takeStorageNotice: () => takeStorageNotice(),
  },
}));

describe("SettingsListWindow", () => {
  beforeEach(() => {
    listAccounts.mockResolvedValue([
      {
        id: "acct-1",
        serviceName: "GitHub",
        digits: 6,
        period: 30,
        algorithm: "SHA1",
        sortOrder: 0,
      },
    ]);
    deleteAccount.mockResolvedValue(undefined);
    openAccountEditorWindow.mockResolvedValue(undefined);
    takeStorageNotice.mockResolvedValue(null);
  });

  it("deletes an account from the list", async () => {
    const user = userEvent.setup();
    render(<SettingsListWindow />);

    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: /delete/i }));

    await waitFor(() => expect(deleteAccount).toHaveBeenCalledWith("acct-1"));
  });
});
