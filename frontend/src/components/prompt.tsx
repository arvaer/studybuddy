// The operator writes prompts in Markdown with TeX math (`$G_t$`, `$$…$$`),
// as the learning capsule tells it to (backend/capsules/learning.capsule,
// coach/system). This renders that, inline in whatever element wraps it,
// so a plain sentence is still just a sentence.
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import "katex/dist/katex.min.css";
import { HighlightedCode } from "./highlighted-code";
import { codeLanguage } from "@/lib/code-languages";

/// remark-math only treats `$$` as display math when it sits on its own
/// line; a line that is nothing but `$$…$$` is meant as display too.
function displayMath(text: string): string {
  return text.replace(/^[ \t]*\$\$([^\n]+?)\$\$[ \t]*$/gm, "$$$$\n$1\n$$$$");
}

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
        code: ({ className, children }) => {
          // A fenced block in a known language is colored; react-markdown
          // marks it with `language-<info>`.
          const language = codeLanguage(/language-(\S+)/.exec(className ?? "")?.[1] ?? "");
          if (language && typeof children === "string") {
            return <code><HighlightedCode code={children.replace(/\n$/, "")} language={language} /></code>;
          }
          return <code className="bg-muted px-1.5 py-0.5 rounded font-mono text-[0.9em]">{children}</code>;
        },
        // A fenced block: the code inside drops the inline pill, and the
        // block keeps body type even inside the display-font prompt.
        pre: ({ children }) => (
          <pre className="bg-muted rounded-lg p-4 overflow-x-auto mb-3 last:mb-0 font-mono text-sm font-normal leading-normal [&>code]:bg-transparent [&>code]:p-0 [&>code]:text-[1em]">
            {children}
          </pre>
        ),
      }}
    >
      {displayMath(children)}
    </ReactMarkdown>
  );
}
