import { useState, useRef, useEffect } from "react";
import { Send } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

type Message = {
  role: "user" | "assistant" | "system";
  content: string;
};

interface QuizAiChatProps {
  questionPrompt: string;
  questionOptions?: string[];
  correctAnswer: string;
  explanation: string;
  userAnswer?: string;
  wasCorrect?: boolean;
}

const mockAiRespond = (messages: Message[]): string => {
  const lastUser = messages.filter(m => m.role === "user").pop()?.content.toLowerCase() || "";
  if (lastUser.includes("why") || lastUser.includes("explain")) {
    return "Great question! The key insight here is understanding the underlying concept. The correct answer follows from the definition — try reviewing the explanation shown after answering, and let me know if a specific part is still unclear.";
  }
  if (lastUser.includes("hint") || lastUser.includes("help")) {
    return "Here's a hint: think about the core definition and how it applies in this specific context. Try eliminating options that clearly don't fit first.";
  }
  return "I'm here to help you understand this question better. You can ask me things like \"Why is that the answer?\" or \"Can you explain the concept?\"";
};

export function QuizAiChat({
  questionPrompt,
  correctAnswer,
  explanation,
  userAnswer,
  wasCorrect,
}: QuizAiChatProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Reset when question changes
  useEffect(() => {
    setMessages([]);
    setInput("");
  }, [questionPrompt]);

  const contextSummary = userAnswer
    ? `You answered "${userAnswer}" which was ${wasCorrect ? "correct" : "incorrect"}. The correct answer is "${correctAnswer}". ${explanation}`
    : `The question is: "${questionPrompt}"`;

  const handleSend = () => {
    if (!input.trim()) return;

    const userMsg: Message = { role: "user", content: input.trim() };
    const systemCtx: Message = { role: "system", content: `Context: ${contextSummary}` };
    const updated = [...messages, userMsg];
    setMessages(updated);
    setInput("");
    setIsTyping(true);

    // Mock AI delay
    setTimeout(() => {
      const reply = mockAiRespond([systemCtx, ...updated]);
      setMessages(prev => [...prev, { role: "assistant", content: reply }]);
      setIsTyping(false);
    }, 800 + Math.random() * 600);
  };

  useEffect(() => {
    scrollRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isTyping]);

  return (
    <div className="flex flex-col h-full">
      {/* Context banner */}
      <div className="px-4 py-3 border-b border-border bg-muted/30">
        <p className="text-xs text-muted-foreground font-medium mb-1">Question context</p>
        <p className="text-sm text-foreground line-clamp-2">{questionPrompt}</p>
        {userAnswer && (
          <p className={cn(
            "text-xs mt-1 font-medium",
            wasCorrect ? "text-stable" : "text-unstable"
          )}>
            You answered {wasCorrect ? "correctly" : "incorrectly"}
          </p>
        )}
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1 px-4 py-3">
        <div className="space-y-3">
          {messages.length === 0 && (
            <div className="text-center py-8">
              <p className="text-sm text-muted-foreground">
                Ask anything about this question — why an answer is correct, request a hint, or dig deeper into the concept.
              </p>
            </div>
          )}
          {messages.filter(m => m.role !== "system").map((msg, i) => (
            <div
              key={i}
              className={cn(
                "flex",
                msg.role === "user" ? "justify-end" : "justify-start"
              )}
            >
              <div
                className={cn(
                  "max-w-[85%] rounded-2xl px-4 py-2.5 text-sm",
                  msg.role === "user"
                    ? "bg-primary text-primary-foreground rounded-br-md"
                    : "bg-muted text-foreground rounded-bl-md"
                )}
              >
                {msg.content}
              </div>
            </div>
          ))}
          {isTyping && (
            <div className="flex justify-start">
              <div className="bg-muted rounded-2xl rounded-bl-md px-4 py-2.5 text-sm text-muted-foreground">
                <span className="animate-pulse">Thinking…</span>
              </div>
            </div>
          )}
          <div ref={scrollRef} />
        </div>
      </ScrollArea>

      {/* Input */}
      <div className="p-3 border-t border-border">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSend();
          }}
          className="flex gap-2"
        >
          <Input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask about this question…"
            className="flex-1"
          />
          <Button type="submit" size="icon" disabled={!input.trim() || isTyping}>
            <Send className="h-4 w-4" />
          </Button>
        </form>
      </div>
    </div>
  );
}
