import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { GoalChips } from "./goal-chips";
import type { GoalState } from "@/lib/workspace";

const goal = (revision: number, view: Partial<GoalState>): GoalState => ({
  id: `g${revision}`,
  workspaceId: "ws-1",
  revision,
  intent: `Goal ${revision}`,
  createdAt: "2026-10-04T00:00:00Z",
  operator: "idle",
  ...view,
});

const goals = [
  goal(1, { summary: "Done." }),
  goal(2, { operator: "waiting", currentActivityId: "a2" }),
  goal(3, { operator: "thinking" }),
];

describe("GoalChips", () => {
  it("marks the goal on screen and says which are finished or thinking without words", () => {
    render(<GoalChips goals={goals} selectedId="g2" onSelect={() => {}} adding={false} onAdd={() => {}} />);
    expect(screen.getByRole("button", { name: /Goal 2/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /Goal 1/ })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByRole("button", { name: /finished Goal 1/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /thinking Goal 3/ })).toBeInTheDocument();
  });

  it("switches goals and opens the intent box from the + chip", () => {
    const onSelect = vi.fn();
    const onAdd = vi.fn();
    const { rerender } = render(<GoalChips goals={goals} selectedId="g2" onSelect={onSelect} adding={false} onAdd={onAdd} />);
    fireEvent.click(screen.getByRole("button", { name: /Goal 1/ }));
    expect(onSelect).toHaveBeenCalledWith("g1");
    fireEvent.click(screen.getByRole("button", { name: "New goal" }));
    expect(onAdd).toHaveBeenCalled();

    // While adding, the + chip is the one marked.
    rerender(<GoalChips goals={goals} selectedId="g2" onSelect={onSelect} adding onAdd={onAdd} />);
    expect(screen.getByRole("button", { name: "New goal" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /Goal 2/ })).toHaveAttribute("aria-pressed", "false");
  });
});
