// The languages a fenced code block is colored in, by its info string
// (```python). One Lezer grammar per language serves both the answer editor
// and rendered Markdown, so code looks the same while typed and once shown.
import { pythonLanguage } from "@codemirror/lang-python";
import type { Language } from "@codemirror/language";

const LANGUAGES: Record<string, Language> = {
  python: pythonLanguage,
  py: pythonLanguage,
};

export function codeLanguage(info: string): Language | null {
  return LANGUAGES[info.trim().toLowerCase()] ?? null;
}
