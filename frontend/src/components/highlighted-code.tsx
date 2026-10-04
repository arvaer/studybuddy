// Code colored by its Lezer grammar into `tok-*` classes (index.css), the
// same classes the answer editor gives code as it is typed.
import type { ReactNode } from "react";
import { classHighlighter, highlightCode } from "@lezer/highlight";
import type { Language } from "@codemirror/language";

export function HighlightedCode({ code, language }: { code: string; language: Language }) {
  const out: ReactNode[] = [];
  highlightCode(
    code,
    language.parser.parse(code),
    classHighlighter,
    (text, classes) => out.push(classes ? <span key={out.length} className={classes}>{text}</span> : text),
    () => out.push("\n"),
  );
  return <>{out}</>;
}
