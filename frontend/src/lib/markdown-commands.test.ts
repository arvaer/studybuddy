import { describe, expect, it } from "vitest";
import { EditorSelection, EditorState, type TransactionSpec } from "@codemirror/state";
import { block, list, wrap } from "./markdown-commands";

/// Apply a command to `text` with `[` and `]` marking the selection; return
/// the result marked the same way.
function run(text: string, command: (s: EditorState) => TransactionSpec): string {
  const from = text.indexOf("[");
  const to = text.indexOf("]") - 1;
  const state = EditorState.create({
    doc: text.replace("[", "").replace("]", ""),
    selection: EditorSelection.single(from, to),
  });
  const next = state.update(command(state)).state;
  const doc = next.doc.toString();
  const sel = next.selection.main;
  return doc.slice(0, sel.from) + "[" + doc.slice(sel.from, sel.to) + "]" + doc.slice(sel.to);
}

describe("markdown commands", () => {
  it("wraps the selection, and unwraps it on a second press", () => {
    expect(run("a [word] b", (s) => wrap(s, "**", "bold"))).toBe("a **[word]** b");
    expect(run("a **[word]** b", (s) => wrap(s, "**", "bold"))).toBe("a [word] b");
  });

  it("inserts a selected placeholder when nothing is selected", () => {
    expect(run("so []", (s) => wrap(s, "$", "x"))).toBe("so $[x]$");
  });

  it("puts a block on lines of its own", () => {
    expect(run("see [x = 1] here", (s) => block(s, "```python", "```", "code"))).toBe("see \n```python\n[x = 1]\n```\n here");
    expect(run("[]", (s) => block(s, "$$", "$$", "x"))).toBe("$$\n[x]\n$$");
  });

  it("numbers the selected lines, and takes the numbers off again", () => {
    expect(run("[a\nb]", (s) => list(s, true))).toBe("1. [a\n2. b]");
    expect(run("[1. a\n2. b]", (s) => list(s, true))).toBe("[a\nb]");
    expect(run("[a]", (s) => list(s, false))).toBe("- [a]");
  });
});
