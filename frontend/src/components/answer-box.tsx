// The learner's answer, written the way the coach writes prompts: Markdown
// with TeX math (`$V(s)$`, `$$…$$`) and fenced code. While drafting, a
// preview renders under the box once the draft uses any of that; once
// accepted, the answer is shown rendered rather than as raw source.
import type { KeyboardEvent } from "react";
import { Prompt } from "./prompt";
import { Textarea } from "@/components/ui/textarea";

interface AnswerBoxProps {
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  /// The accepted answer: rendered, not editable.
  accepted: boolean;
}

/// Whether a draft uses anything the preview would render differently.
function hasFormatting(text: string): boolean {
  return /[$`*_#|]|^\s*([-+]|\d+\.)\s/m.test(text);
}

/// Inside an open ``` fence, Tab indents instead of leaving the box.
function insideFence(text: string, caret: number): boolean {
  return (text.slice(0, caret).match(/^\s*```/gm) ?? []).length % 2 === 1;
}

export function AnswerBox({ value, onChange, disabled, accepted }: AnswerBoxProps) {
  if (accepted) {
    return (
      <div className="rounded-md border border-border bg-muted/30 px-4 py-3 text-foreground" data-testid="accepted-answer">
        <Prompt>{value}</Prompt>
      </div>
    );
  }

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    const box = e.currentTarget;
    if (e.key !== "Tab" || e.shiftKey || !insideFence(box.value, box.selectionStart)) return;
    e.preventDefault();
    const { selectionStart: start, selectionEnd: end } = box;
    onChange(box.value.slice(0, start) + "    " + box.value.slice(end));
    requestAnimationFrame(() => box.setSelectionRange(start + 4, start + 4));
  };

  return (
    <div className="space-y-2">
      <Textarea
        aria-label="Your answer"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        disabled={disabled}
        placeholder="Write your answer"
        rows={5}
        className="font-[inherit]"
      />
      <p className="text-xs text-muted-foreground">
        Markdown works: <code className="font-mono">$math$</code>, <code className="font-mono">$$display$$</code>,{" "}
        <code className="font-mono">```code```</code>, lists.
      </p>
      {hasFormatting(value) && (
        <div className="rounded-md border border-dashed border-border px-4 py-3 text-foreground" data-testid="answer-preview">
          <div className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">Preview</div>
          <Prompt>{value}</Prompt>
        </div>
      )}
    </div>
  );
}
