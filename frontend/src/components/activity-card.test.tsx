import { act, render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ActivityCard } from "./activity-card";
import { ApiError } from "@/lib/api";
import type { AttemptReceipt, Revision } from "@/lib/activities";
import { memoryStorage, type AttemptClient } from "@/lib/attempt-state";
import { EditorView } from "@codemirror/view";

const revision: Revision = {
  id: "rev-1",
  activityId: "act-1",
  revision: 1,
  prompt: "Which update rule is on-policy?",
  options: ["Q-learning", "SARSA", "DQN"],
  hasAnswerKey: true,
  sourceResourceId: null,
  sourceArtifactId: null,
  sourceLocation: null,
  createdAt: "2026-09-30T00:00:00Z",
};

function receipt(status: AttemptReceipt["status"]): AttemptReceipt {
  return { attemptId: "att-1", activityRevisionId: "rev-1", submittedAt: "2026-09-30T00:00:00Z", status, response: "SARSA", assessment: null };
}

function client(over: Partial<AttemptClient> = {}): AttemptClient {
  return {
    record: vi.fn(async () => ({ receipt: receipt("correct"), replayed: false })),
    fetch: vi.fn(async () => receipt("correct")),
    newKey: () => "key-1",
    ...over,
  };
}

/// Replace the answer editor's text, cursor at the end, as typing would.
function typeAnswer(text: string): EditorView {
  const view = EditorView.findFromDOM(screen.getByLabelText("Your answer"))!;
  act(() => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: { anchor: text.length } }));
  return view;
}

const phaseOf = () => screen.getByText(revision.prompt).closest("[data-phase]")!.getAttribute("data-phase");

describe("ActivityCard", () => {
  it("renders the accepted receipt after submitting through the backend", async () => {
    const c = client();
    render(<ActivityCard revision={revision} deps={{ storage: memoryStorage(), client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));

    fireEvent.click(screen.getByRole("radio", { name: /SARSA/ }));
    expect(screen.getByText("Draft, not submitted")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Submit answer" }));

    await waitFor(() => expect(phaseOf()).toBe("accepted"));
    expect(screen.getByText("Correct")).toBeInTheDocument();
    expect(c.record).toHaveBeenCalledWith({ requestKey: "key-1", activityRevisionId: "rev-1", response: "SARSA", assistance: [] });
    expect(screen.queryByRole("button", { name: "Submit answer" })).not.toBeInTheDocument();
  });

  it("shows pending as recorded and awaiting, not as wrong", async () => {
    const c = client({ record: vi.fn(async () => ({ receipt: receipt("pending"), replayed: false })) });
    render(<ActivityCard revision={{ ...revision, options: null, hasAnswerKey: false }} deps={{ storage: memoryStorage(), client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));

    typeAnswer("It bootstraps from the action actually taken.");
    fireEvent.click(screen.getByRole("button", { name: "Submit answer" }));

    await waitFor(() => expect(phaseOf()).toBe("accepted"));
    expect(screen.getByText("Recorded, awaiting assessment")).toBeInTheDocument();
    expect(screen.queryByText(/Not quite/)).not.toBeInTheDocument();
  });

  it("offers a retry after a network failure that resends the same request key", async () => {
    const record = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce({ receipt: receipt("incorrect"), replayed: true });
    const c = client({ record });
    render(<ActivityCard revision={revision} deps={{ storage: memoryStorage(), client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));

    fireEvent.click(screen.getByRole("radio", { name: /DQN/ }));
    fireEvent.click(screen.getByRole("button", { name: "Submit answer" }));
    await waitFor(() => expect(phaseOf()).toBe("failed"));
    expect(screen.getByText("Not submitted: Failed to fetch")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(phaseOf()).toBe("accepted"));
    expect(screen.getByText("Not quite right")).toBeInTheDocument();
    expect(screen.getByText("(already recorded)")).toBeInTheDocument();
    expect(record.mock.calls.map((call) => call[0].requestKey)).toEqual(["key-1", "key-1"]);
  });

  it("does not offer a retry for a rejected payload", async () => {
    const c = client({ record: vi.fn().mockRejectedValue(new ApiError(422, "response must be one of the options")) });
    render(<ActivityCard revision={revision} deps={{ storage: memoryStorage(), client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));

    fireEvent.click(screen.getByRole("radio", { name: /DQN/ }));
    fireEvent.click(screen.getByRole("button", { name: "Submit answer" }));

    await waitFor(() => expect(phaseOf()).toBe("failed"));
    expect(screen.getByText("Rejected: response must be one of the options")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Retry" })).not.toBeInTheDocument();
  });

  it("restores the accepted attempt from the backend after a remount, as a refresh would", async () => {
    const storage = memoryStorage();
    const c = client();
    const first = render(<ActivityCard revision={revision} deps={{ storage, client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));
    fireEvent.click(screen.getByRole("radio", { name: /SARSA/ }));
    fireEvent.click(screen.getByRole("button", { name: "Submit answer" }));
    await waitFor(() => expect(phaseOf()).toBe("accepted"));
    first.unmount();

    render(<ActivityCard revision={revision} deps={{ storage, client: c }} />);
    expect(phaseOf()).toBe("restoring");
    expect(screen.getByText("Loading your attempt")).toBeInTheDocument();
    await waitFor(() => expect(phaseOf()).toBe("accepted"));
    expect(c.fetch).toHaveBeenCalledWith("att-1");
    expect(c.record).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("radio", { name: /SARSA/ })).toHaveAttribute("aria-checked", "true");
  });

  it("keeps an unsent draft across a remount", async () => {
    const storage = memoryStorage();
    const c = client();
    const first = render(<ActivityCard revision={revision} deps={{ storage, client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));
    await act(async () => fireEvent.click(screen.getByRole("radio", { name: /Q-learning/ })));
    first.unmount();

    render(<ActivityCard revision={revision} deps={{ storage, client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));
    expect(screen.getByRole("radio", { name: /Q-learning/ })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByText("Draft, not submitted")).toBeInTheDocument();
    expect(c.record).not.toHaveBeenCalled();
  });

  it("renders the prompt as markdown with math", () => {
    const c = client();
    const math = { ...revision, id: "rev-m", prompt: "Use $G_t = R_{t+1} + \\gamma G_{t+1}$ with **three** steps:\n\n- $R_1 = 1$\n- $R_2 = 0$" };
    const { container } = render(<ActivityCard revision={math} deps={{ storage: memoryStorage(), client: c }} />);
    expect(container.querySelectorAll(".katex").length).toBe(3);
    expect(screen.getByText("three").tagName).toBe("STRONG");
    expect(container.querySelectorAll("li").length).toBe(2);
    expect(container.textContent).not.toContain("$");
  });

  it("draws math in the answer as it is typed, and shows source where the cursor is", async () => {
    const answer = "It is $v_\\pi(s)$ here.";
    const c = client({ record: vi.fn(async () => ({ receipt: { ...receipt("pending"), response: answer }, replayed: false })) });
    render(<ActivityCard revision={{ ...revision, options: null, hasAnswerKey: false }} deps={{ storage: memoryStorage(), client: c }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));
    const editor = typeAnswer(answer);
    expect(editor.dom.querySelector(".cm-math .katex")).toBeTruthy();
    act(() => editor.dispatch({ selection: { anchor: answer.indexOf("v_") } }));
    expect(editor.dom.querySelector(".cm-math")).toBeNull();
    // A line that is only $$…$$ is display math.
    typeAnswer("so\n$$v = 1$$\nok");
    expect(editor.dom.querySelector(".cm-math-display")).toBeTruthy();

    typeAnswer(answer);
    fireEvent.click(screen.getByRole("button", { name: "Submit answer" }));
    await waitFor(() => expect(phaseOf()).toBe("accepted"));
    expect(c.record).toHaveBeenCalledWith(expect.objectContaining({ response: answer }));
    expect(screen.getByTestId("accepted-answer").querySelector(".katex")).toBeTruthy();
    expect(screen.queryByLabelText("Your answer")).toBeNull();
  });

  it("indents with Tab inside a code fence and leaves Tab alone outside one", async () => {
    render(<ActivityCard revision={{ ...revision, options: null, hasAnswerKey: false }} deps={{ storage: memoryStorage(), client: client() }} />);
    await waitFor(() => expect(phaseOf()).toBe("draft"));
    const editor = typeAnswer("plain");
    expect(fireEvent.keyDown(editor.contentDOM, { key: "Tab" })).toBe(true);
    const fenced = "```python\ndef f():\n";
    typeAnswer(fenced);
    expect(fireEvent.keyDown(editor.contentDOM, { key: "Tab" })).toBe(false);
    expect(editor.state.doc.toString()).toBe(fenced + "    ");
  });

  it("shows the cited page of the source under the prompt", async () => {
    const c = client();
    const cited = { ...revision, id: "rev-c", sourceResourceId: "res-1", sourceLocation: { page: 58 } };
    const loadPage = vi.fn(async () => "The Markov property is best viewed as a restriction on the state.");
    render(<ActivityCard revision={cited} deps={{ storage: memoryStorage(), client: c }} loadPage={loadPage} />);
    expect(screen.getByText(/Source · p\. 58/)).toBeTruthy();
    await waitFor(() => expect(screen.getByText(/restriction on the state/)).toBeTruthy());
    expect(loadPage).toHaveBeenCalledWith("res-1", 58);
    // Once the drawn page arrives it replaces the text; the text stays a click away.
    const img = screen.getByAltText("Page 58 of the source") as HTMLImageElement;
    expect(img.getAttribute("src")).toBe("/api/resources/res-1/pages/58/image");
    fireEvent.load(img);
    expect(screen.queryByText(/restriction on the state/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "text" }));
    expect(screen.getByText(/restriction on the state/)).toBeTruthy();
    // An uncited revision shows no passage.
    const { container } = render(<ActivityCard revision={{ ...revision, id: "rev-u" }} deps={{ storage: memoryStorage(), client: c }} />);
    expect(container.querySelector("[data-testid=source-passage]")).toBeNull();
  });

  it("asks the coach for a hint, shows it, and records it as assistance", async () => {
    vi.useFakeTimers();
    try {
      const c = client();
      const hint = { id: "h1", text: "Think about what the discount does.", createdAt: "2026-10-03T00:00:00Z" };
      let hints: typeof hint[] = [];
      const hintClient = {
        request: vi.fn(async () => { hints = [hint]; }),
        fetch: vi.fn(async () => hints),
      };
      const free = { ...revision, id: "rev-h", options: null, hasAnswerKey: false };
      render(<ActivityCard revision={free} deps={{ storage: memoryStorage(), client: c }} hintClient={hintClient} />);
      await act(async () => { await Promise.resolve(); });
      typeAnswer("the sum of");
      await act(async () => { fireEvent.click(screen.getByRole("button", { name: /hint/i })); });
      expect(hintClient.request).toHaveBeenCalledWith("rev-h", "the sum of");
      await act(async () => { await vi.advanceTimersByTimeAsync(2100); });
      expect(screen.getByText(/Think about what the discount does/)).toBeTruthy();
      await act(async () => { fireEvent.click(screen.getByText("Submit answer")); });
      await act(async () => { await Promise.resolve(); });
      expect(c.record).toHaveBeenCalledWith(expect.objectContaining({
        response: "the sum of",
        assistance: [{ kind: "hint", text: hint.text }],
      }));
    } finally {
      vi.useRealTimers();
    }
  });
});
