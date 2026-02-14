import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

export function MarkdownRenderer({ content, className }: MarkdownRendererProps) {
  return (
    <div className={cn("prose prose-slate dark:prose-invert max-w-none", className)}>
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        h1: ({ children }) => (
          <h1 className="font-display text-3xl font-bold mt-0 mb-6">{children}</h1>
        ),
        h2: ({ children }) => (
          <h2 className="font-display text-2xl font-semibold mt-8 mb-4">{children}</h2>
        ),
        h3: ({ children }) => (
          <h3 className="font-display text-xl font-medium mt-6 mb-3">{children}</h3>
        ),
        h4: ({ children }) => (
          <h4 className="font-display text-lg font-medium mt-4 mb-2">{children}</h4>
        ),
        blockquote: ({ children }) => (
          <blockquote className="border-l-4 border-accent pl-4 my-4 text-muted-foreground italic">
            {children}
          </blockquote>
        ),
        code: ({ className: codeClassName, children, ...props }) => {
          const isInline = !codeClassName;
          if (isInline) {
            return (
              <code className="bg-muted px-1.5 py-0.5 rounded text-sm font-mono" {...props}>
                {children}
              </code>
            );
          }
          return (
            <code className={cn("block bg-muted p-4 rounded-lg text-sm font-mono overflow-x-auto", codeClassName)} {...props}>
              {children}
            </code>
          );
        },
        pre: ({ children }) => (
          <pre className="bg-muted rounded-lg overflow-x-auto my-4">{children}</pre>
        ),
        p: ({ children }) => {
          // Detect KaTeX blocks: $$...$$ for display math
          const text = typeof children === "string" ? children : null;
          if (text && text.startsWith("$$") && text.endsWith("$$")) {
            const math = text.slice(2, -2).trim();
            try {
              const katex = require("katex");
              return (
                <div
                  className="my-4 overflow-x-auto text-center"
                  dangerouslySetInnerHTML={{
                    __html: katex.renderToString(math, { displayMode: true }),
                  }}
                />
              );
            } catch {
              return <p className="mb-4 font-mono text-sm">{text}</p>;
            }
          }
          return <p className="mb-4 leading-relaxed">{children}</p>;
        },
      }}
    >
      {content}
    </ReactMarkdown>
    </div>
  );
}
