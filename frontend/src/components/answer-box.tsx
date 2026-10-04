// The learner's answer, written the way the coach writes prompts: Markdown
// with TeX math (`$V(s)$`, `$$…$$`) and fenced code. Math renders in place
// while drafting (math-editor); once accepted, the whole answer is shown
// rendered rather than as source.
import { MathEditor } from "./math-editor";
import { Prompt } from "./prompt";

interface AnswerBoxProps {
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  /// The accepted answer: rendered, not editable.
  accepted: boolean;
}

export function AnswerBox({ value, onChange, disabled, accepted }: AnswerBoxProps) {
  if (accepted) {
    return (
      <div className="rounded-md border border-border bg-muted/30 px-4 py-3 text-foreground" data-testid="accepted-answer">
        <Prompt>{value}</Prompt>
      </div>
    );
  }
  return (
    <MathEditor
      ariaLabel="Your answer"
      value={value}
      onChange={onChange}
      disabled={disabled}
      placeholder="Write your answer. $math$ renders as you type."
    />
  );
}
