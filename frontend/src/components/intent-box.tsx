import { useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

// The one text box (20a): the learner says what they want to learn, and the
// operator goes. No mode picker, no onboarding.

interface IntentBoxProps {
  onSubmit: (intent: string) => Promise<void>;
}

export function IntentBox({ onSubmit }: IntentBoxProps) {
  const [intent, setIntent] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    const text = intent.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onSubmit(text);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "could not start");
      setBusy(false);
    }
  };

  return (
    <div className="w-full max-w-xl mx-auto flex flex-col gap-3">
      <h1 className="font-display text-2xl font-semibold text-foreground">What do you want to learn?</h1>
      <Textarea
        value={intent}
        onChange={(e) => setIntent(e.target.value)}
        placeholder="e.g. Understand the return, discounting and the Bellman equation well enough to derive Q-learning."
        rows={4}
        disabled={busy}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) void submit();
        }}
      />
      {error && <p className="text-sm text-unstable">{error}</p>}
      <div className="flex items-center justify-between">
        <p className="text-xs text-muted-foreground">The operator reads this and puts the first activity in front of you.</p>
        <Button onClick={() => void submit()} disabled={busy || !intent.trim()}>
          {busy ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : null}
          Go
        </Button>
      </div>
    </div>
  );
}
