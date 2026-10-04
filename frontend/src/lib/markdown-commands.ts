// What the answer editor's toolbar and shortcuts do to the text. Each takes
// the editor state and returns the change to apply, so it is tested without
// a browser. Wrapping toggles: applied twice, a mark comes off again.
import { EditorSelection, type EditorState, type TransactionSpec } from "@codemirror/state";

/// Put `mark` around the selection (or around `fallback`, selected, when
/// nothing is), or take it off when the selection already sits inside it.
export function wrap(state: EditorState, mark: string, fallback: string, close = mark): TransactionSpec {
  return state.changeByRange((range) => {
    const before = state.sliceDoc(range.from - mark.length, range.from);
    const after = state.sliceDoc(range.to, range.to + close.length);
    if (before === mark && after === close) {
      return {
        changes: [
          { from: range.from - mark.length, to: range.from },
          { from: range.to, to: range.to + close.length },
        ],
        range: EditorSelection.range(range.from - mark.length, range.to - mark.length),
      };
    }
    const inner = range.empty ? fallback : state.sliceDoc(range.from, range.to);
    return {
      changes: { from: range.from, to: range.to, insert: mark + inner + close },
      range: EditorSelection.range(range.from + mark.length, range.from + mark.length + inner.length),
    };
  });
}

/// A block on lines of its own: `open`, the selection (or `fallback`,
/// selected), `close`.
export function block(state: EditorState, open: string, close: string, fallback: string): TransactionSpec {
  return state.changeByRange((range) => {
    const inner = range.empty ? fallback : state.sliceDoc(range.from, range.to);
    const lineStart = state.doc.lineAt(range.from).from === range.from;
    const lineEnd = state.doc.lineAt(range.to).to === range.to;
    const lead = (lineStart ? "" : "\n") + open + "\n";
    const insert = lead + inner + "\n" + close + (lineEnd ? "" : "\n");
    return {
      changes: { from: range.from, to: range.to, insert },
      range: EditorSelection.range(range.from + lead.length, range.from + lead.length + inner.length),
    };
  });
}

/// Start every selected line with a list marker, or take it off when all
/// of them already have it. `numbered` counts 1., 2., ….
export function list(state: EditorState, numbered: boolean): TransactionSpec {
  const marker = numbered ? /^(\s*)\d+\. / : /^(\s*)[-*+] /;
  const { from, to } = state.selection.main;
  const first = state.doc.lineAt(from).number;
  const last = state.doc.lineAt(to).number;
  const lines = Array.from({ length: last - first + 1 }, (_, i) => state.doc.line(first + i));
  const off = lines.every((l) => marker.test(l.text));
  const changes = lines.map((l, i) => {
    const match = marker.exec(l.text);
    if (off && match) return { from: l.from + match[1].length, to: l.from + match[0].length };
    return { from: l.from, insert: numbered ? `${i + 1}. ` : "- " };
  });
  return { changes };
}
