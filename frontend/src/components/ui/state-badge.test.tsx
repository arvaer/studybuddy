import { render, screen } from "@testing-library/react";
import { StateBadge } from "./state-badge";
import type { RUState } from "@/types/study";

const states: RUState[] = ["introduced", "reinforced", "unstable", "stabilizing", "stable", "superseded"];

describe("StateBadge", () => {
  it("names the stored state and never claims mastery or a due date", () => {
    render(
      <>
        {states.map((s) => (
          <StateBadge key={s} state={s} />
        ))}
      </>
    );
    expect(screen.getByText("Stable")).toBeInTheDocument();
    expect(screen.queryByText(/master/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/\bdue\b/i)).not.toBeInTheDocument();
  });
});
