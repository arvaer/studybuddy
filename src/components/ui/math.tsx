import 'katex/dist/katex.min.css';
import { BlockMath, InlineMath } from 'react-katex';
import { cn } from '@/lib/utils';

interface MathProps {
  children: string;
  className?: string;
}

/**
 * Renders inline LaTeX math equations.
 * Usage: <InlineMathDisplay>E = mc^2</InlineMathDisplay>
 */
export function InlineMathDisplay({ children, className }: MathProps) {
  return (
    <span className={cn("inline", className)}>
      <InlineMath math={children} />
    </span>
  );
}

/**
 * Renders block-level LaTeX math equations (centered, on their own line).
 * Usage: <BlockMathDisplay>\int_0^1 x^2 dx = \frac{1}{3}</BlockMathDisplay>
 */
export function BlockMathDisplay({ children, className }: MathProps) {
  return (
    <div className={cn("my-4", className)}>
      <BlockMath math={children} />
    </div>
  );
}

/**
 * Parses text containing LaTeX delimiters and renders math inline.
 * Supports $...$ for inline math and $$...$$ for block math.
 * Usage: <MathText>The equation $E = mc^2$ is famous.</MathText>
 */
export function MathText({ children, className }: MathProps) {
  // Parse the text for LaTeX delimiters
  const parts: React.ReactNode[] = [];
  let remaining = children;
  let key = 0;

  while (remaining.length > 0) {
    // Check for block math first ($$...$$)
    const blockMatch = remaining.match(/\$\$([\s\S]*?)\$\$/);
    // Check for inline math ($...$)
    const inlineMatch = remaining.match(/\$([^$]+)\$/);

    if (blockMatch && (!inlineMatch || blockMatch.index! <= inlineMatch.index!)) {
      // Add text before the match
      if (blockMatch.index! > 0) {
        parts.push(<span key={key++}>{remaining.slice(0, blockMatch.index)}</span>);
      }
      // Add the block math
      parts.push(<BlockMathDisplay key={key++}>{blockMatch[1]}</BlockMathDisplay>);
      remaining = remaining.slice(blockMatch.index! + blockMatch[0].length);
    } else if (inlineMatch) {
      // Add text before the match
      if (inlineMatch.index! > 0) {
        parts.push(<span key={key++}>{remaining.slice(0, inlineMatch.index)}</span>);
      }
      // Add the inline math
      parts.push(<InlineMathDisplay key={key++}>{inlineMatch[1]}</InlineMathDisplay>);
      remaining = remaining.slice(inlineMatch.index! + inlineMatch[0].length);
    } else {
      // No more math, add the rest as text
      parts.push(<span key={key++}>{remaining}</span>);
      break;
    }
  }

  return <span className={cn("", className)}>{parts}</span>;
}
