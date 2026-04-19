import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TotpRow } from "./TotpRow";

describe("TotpRow", () => {
  it("renders grouped code and copy action", async () => {
    const user = userEvent.setup();
    const onCopy = vi.fn();

    render(
      <TotpRow
        code={{
          id: "github",
          serviceName: "GitHub",
          accountLabel: "dev@example.com",
          formattedCode: "123 456",
          rawCode: "123456",
          secondsRemaining: 14,
          period: 30,
        }}
        isCopied={false}
        onCopy={onCopy}
      />,
    );

    expect(screen.getByText("GitHub")).toBeInTheDocument();
    expect(screen.getByText("123 456")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Copy GitHub code" }));

    expect(onCopy).toHaveBeenCalledWith("github");
  });
});
