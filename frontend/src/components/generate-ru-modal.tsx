import { useState } from "react";
import { Loader2, Sparkles, Check } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import {
  callLlm,
  buildRuGenerationMessages,
  parseRuGenerationResponse,
  LlmNotConfiguredError,
  type GeneratedRu,
} from "@/lib/llm";
import { createRUs } from "@/lib/api";
import type { Concept, ReinforcementUnit } from "@/types/study";

interface GenerateRuModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  resourceId: string;
  contentText: string;
  concepts: Concept[];
  onCreated: (rus: ReinforcementUnit[]) => void;
}

type Step = "configure" | "generating" | "review" | "saving";

export function GenerateRuModal({
  open,
  onOpenChange,
  resourceId,
  contentText,
  concepts,
  onCreated,
}: GenerateRuModalProps) {
  const [conceptId, setConceptId] = useState<string>(concepts[0]?.id ?? "");
  const [step, setStep] = useState<Step>("configure");
  const [generated, setGenerated] = useState<(GeneratedRu & { checked: boolean })[]>([]);

  const selectedConcept = concepts.find((c) => c.id === conceptId);

  const handleGenerate = async () => {
    if (!conceptId || !selectedConcept) return;

    setStep("generating");
    try {
      const messages = buildRuGenerationMessages(
        contentText,
        selectedConcept.name,
        selectedConcept.description
      );
      const raw = await callLlm({ messages, maxTokens: 4096 });
      const parsed = parseRuGenerationResponse(raw);
      if (parsed.length === 0) {
        toast.error("No study cards could be parsed from the AI response.");
        setStep("configure");
        return;
      }
      setGenerated(parsed.map((ru) => ({ ...ru, checked: true })));
      setStep("review");
    } catch (err) {
      if (err instanceof LlmNotConfiguredError) {
        toast.error("AI provider not configured. Go to Settings to set it up.");
      } else {
        toast.error(err instanceof Error ? err.message : "Generation failed");
      }
      setStep("configure");
    }
  };

  const handleSave = async () => {
    const selected = generated.filter((g) => g.checked);
    if (selected.length === 0) {
      toast.error("Select at least one study card to save.");
      return;
    }
    setStep("saving");
    try {
      const items = selected.map((g) => ({ claim: g.claim, context: g.context }));
      const created = await createRUs(conceptId, items, resourceId);
      toast.success(`${created.length} study card${created.length === 1 ? "" : "s"} created`);
      onCreated(created);
      onOpenChange(false);
      // Reset for next open
      setStep("configure");
      setGenerated([]);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save");
      setStep("review");
    }
  };

  const toggleItem = (index: number) => {
    setGenerated((prev) =>
      prev.map((item, i) => (i === index ? { ...item, checked: !item.checked } : item))
    );
  };

  const checkedCount = generated.filter((g) => g.checked).length;
  const preview = contentText.slice(0, 500) + (contentText.length > 500 ? "..." : "");

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        if (!v) {
          setStep("configure");
          setGenerated([]);
        }
        onOpenChange(v);
      }}
    >
      <DialogContent className="max-w-2xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="font-display flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-accent" />
            Generate Study Cards
          </DialogTitle>
          <DialogDescription>
            AI will analyze the resource content and create reinforcement units.
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4 py-2">
          {step === "configure" && (
            <>
              <div className="rounded-lg bg-muted/50 p-3 text-sm text-muted-foreground max-h-32 overflow-y-auto">
                {preview}
              </div>

              <div className="space-y-2">
                <Label>Concept</Label>
                <Select value={conceptId} onValueChange={setConceptId}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select a concept" />
                  </SelectTrigger>
                  <SelectContent>
                    {concepts.map((c) => (
                      <SelectItem key={c.id} value={c.id}>
                        {c.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </>
          )}

          {step === "generating" && (
            <div className="flex flex-col items-center justify-center py-12 gap-3">
              <Loader2 className="h-8 w-8 animate-spin text-accent" />
              <p className="text-sm text-muted-foreground">Generating study cards...</p>
            </div>
          )}

          {(step === "review" || step === "saving") && (
            <div className="space-y-2">
              <p className="text-sm text-muted-foreground">
                {generated.length} card{generated.length === 1 ? "" : "s"} generated.
                Uncheck any you don't want to save.
              </p>
              {generated.map((item, i) => (
                <label
                  key={i}
                  className="flex items-start gap-3 rounded-lg border p-3 cursor-pointer hover:bg-muted/30 transition-colors"
                >
                  <Checkbox
                    checked={item.checked}
                    onCheckedChange={() => toggleItem(i)}
                    className="mt-0.5"
                  />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-foreground leading-snug">
                      {item.claim}
                    </p>
                    {item.context && (
                      <p className="text-xs text-muted-foreground mt-1">{item.context}</p>
                    )}
                  </div>
                </label>
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          {step === "configure" && (
            <Button onClick={handleGenerate} disabled={!conceptId || concepts.length === 0}>
              <Sparkles className="h-4 w-4 mr-1.5" />
              Generate
            </Button>
          )}
          {(step === "review" || step === "saving") && (
            <>
              <Button
                variant="outline"
                onClick={() => {
                  setStep("configure");
                  setGenerated([]);
                }}
                disabled={step === "saving"}
              >
                Back
              </Button>
              <Button onClick={handleSave} disabled={checkedCount === 0 || step === "saving"}>
                {step === "saving" ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-1.5" />
                ) : (
                  <Check className="h-4 w-4 mr-1.5" />
                )}
                Save {checkedCount} Card{checkedCount === 1 ? "" : "s"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
