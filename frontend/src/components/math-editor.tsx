// A Markdown editor with live math, after Obsidian's live preview: TeX
// renders in place as you write it and turns back into source when the
// cursor enters it. `$` pairs itself (backticks do not, so a ``` fence
// types cleanly); code fences read as code and take Tab as an indent;
// everywhere else Tab leaves the box as usual.
import { useEffect, useRef } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { Compartment, EditorState, RangeSetBuilder, StateField, type Range } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, keymap, placeholder as placeholderExt, type DecorationSet } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { codeRanges, mathSpans } from "@/lib/math-spans";

class MathWidget extends WidgetType {
  constructor(readonly tex: string, readonly display: boolean) {
    super();
  }
  eq(other: MathWidget) {
    return other.tex === this.tex && other.display === this.display;
  }
  toDOM() {
    const el = document.createElement("span");
    el.className = this.display ? "cm-math cm-math-display" : "cm-math";
    katex.render(this.tex, el, { displayMode: this.display, throwOnError: false });
    return el;
  }
  ignoreEvent() {
    return false;
  }
}

const codeLine = Decoration.line({ class: "cm-code-line" });

/// Math the cursor is not in, drawn; lines inside a code fence, marked.
function decorate(state: EditorState): DecorationSet {
  const text = state.doc.toString();
  const touched = (from: number, to: number) => state.selection.ranges.some((r) => r.from <= to && r.to >= from);
  const marks: Range<Decoration>[] = [];
  for (const span of mathSpans(text)) {
    if (!touched(span.from, span.to)) {
      marks.push(Decoration.replace({ widget: new MathWidget(span.tex, span.display) }).range(span.from, span.to));
    }
  }
  for (const { from, to } of codeRanges(text).filter((c) => c.fenced)) {
    for (let n = state.doc.lineAt(from).number; n <= state.doc.lineAt(to).number; n++) {
      marks.push(codeLine.range(state.doc.line(n).from));
    }
  }
  const builder = new RangeSetBuilder<Decoration>();
  for (const m of marks.sort((a, b) => a.from - b.from || a.value.startSide - b.value.startSide)) builder.add(m.from, m.to, m.value);
  return builder.finish();
}

const livePreview = StateField.define<DecorationSet>({
  create: decorate,
  update: (deco, tr) => (tr.docChanged || tr.selection ? decorate(tr.state) : deco),
  provide: (field) => EditorView.decorations.from(field),
});

/// Tab inside an open code fence indents; outside, it is not ours.
const tabInFence = keymap.of([
  {
    key: "Tab",
    run: (view) => {
      const head = view.state.selection.main.head;
      const inFence = codeRanges(view.state.doc.toString()).some((c) => c.fenced && head > c.from && head <= c.to);
      if (!inFence) return false;
      view.dispatch(view.state.replaceSelection("    "));
      return true;
    },
  },
]);

/// The third backtick of a fence goes to the start of its line: Enter keeps
/// a code block's indent, and an indented ``` would not close the fence.
const fenceAtLineStart = EditorView.inputHandler.of((view, from, to, text) => {
  if (text !== "`" || from !== to) return false;
  const line = view.state.doc.lineAt(from);
  const before = view.state.sliceDoc(line.from, from);
  if (!/^\s+``$/.test(before)) return false;
  view.dispatch({ changes: { from: line.from, to, insert: "```" }, selection: { anchor: line.from + 3 } });
  return true;
});

const markdownStyle = HighlightStyle.define([
  { tag: tags.strong, fontWeight: "600" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.heading, fontWeight: "600" },
  { tag: tags.monospace, fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: "0.9em" },
  { tag: tags.processingInstruction, color: "hsl(var(--muted-foreground))" },
]);

const theme = EditorView.theme({
  "&": { fontSize: "0.95rem", backgroundColor: "transparent" },
  "&.cm-focused": { outline: "none" },
  // CodeMirror's base theme sets a monospace font here; prose reads in the page's.
  ".cm-scroller": { fontFamily: "inherit" },
  ".cm-content": { minHeight: "10rem", padding: "0.75rem 0", caretColor: "hsl(var(--foreground))" },
  ".cm-line": { padding: "0 0.75rem", lineHeight: "1.65" },
  ".cm-placeholder": { color: "hsl(var(--muted-foreground))" },
  ".cm-code-line": {
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
    fontSize: "0.85em",
    backgroundColor: "hsl(var(--muted))",
  },
  ".cm-math": { cursor: "text" },
  ".cm-math-display": { display: "block", textAlign: "center" },
  ".cm-math-display .katex-display": { margin: "0.25em 0" },
});

interface MathEditorProps {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  ariaLabel: string;
}

export function MathEditor({ value, onChange, disabled = false, placeholder = "", ariaLabel }: MathEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const editable = useRef(new Compartment());
  const latestOnChange = useRef(onChange);
  latestOnChange.current = onChange;

  useEffect(() => {
    const v = new EditorView({
      parent: host.current!,
      state: EditorState.create({
        doc: value,
        extensions: [
          history(),
          closeBrackets(),
          EditorState.languageData.of(() => [{ closeBrackets: { brackets: ["(", "[", "{", "$"], before: ")]}:;>$" } }]),
          tabInFence,
          fenceAtLineStart,
          keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...historyKeymap]),
          markdown(),
          syntaxHighlighting(markdownStyle),
          EditorView.lineWrapping,
          livePreview,
          theme,
          placeholderExt(placeholder),
          EditorView.contentAttributes.of({ "aria-label": ariaLabel, "aria-multiline": "true" }),
          editable.current.of(EditorView.editable.of(!disabled)),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) latestOnChange.current(u.state.doc.toString());
          }),
        ],
      }),
    });
    view.current = v;
    return () => v.destroy();
    // The editor is made once; value and disabled are synced below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const v = view.current;
    if (v && v.state.doc.toString() !== value) {
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: value } });
    }
  }, [value]);

  useEffect(() => {
    view.current?.dispatch({ effects: editable.current.reconfigure(EditorView.editable.of(!disabled)) });
  }, [disabled]);

  return (
    <div
      ref={host}
      className="rounded-md border border-input bg-background focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2 ring-offset-background"
      data-testid="math-editor"
    />
  );
}
