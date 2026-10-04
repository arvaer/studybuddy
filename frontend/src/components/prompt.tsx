// The operator writes prompts in Markdown with TeX math (`$G_t$`, `$$…$$`),
// as the learning capsule tells it to (backend/capsules/learning.capsule,
// coach/system). This renders that, inline in whatever element wraps it,
// so a plain sentence is still just a sentence.
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import "katex/dist/katex.min.css";

export function Prompt({ children }: { children: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm, remarkMath]}
      rehypePlugins={[rehypeKatex]}
      components={{
        // Paragraphs stack with a small gap; the heading around the prompt
        // sets the type.
        p: ({ children }) => <p className="mb-3 last:mb-0">{children}</p>,
        ul: ({ children }) => <ul className="list-disc pl-6 mb-3 last:mb-0 space-y-1">{children}</ul>,
        ol: ({ children }) => <ol className="list-decimal pl-6 mb-3 last:mb-0 space-y-1">{children}</ol>,
        code: ({ children }) => <code className="bg-muted px-1.5 py-0.5 rounded font-mono text-[0.9em]">{children}</code>,
        pre: ({ children }) => <pre className="bg-muted rounded-lg p-4 overflow-x-auto mb-3 text-sm">{children}</pre>,
      }}
    >
      {children}
    </ReactMarkdown>
  );
}
