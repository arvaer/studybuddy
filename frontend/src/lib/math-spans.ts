// Where the TeX is in a Markdown draft, the way Obsidian finds it: `$$…$$`
// (may span lines) is display math, `$…$` is inline math when the dollars
// hug the TeX (so "$5 and $10" stays prose), and nothing inside a code
// fence or inline code counts.

export interface MathSpan {
  from: number;
  to: number;
  tex: string;
  display: boolean;
}

export interface CodeRange {
  from: number;
  to: number;
  /// A ``` block (fence lines included), rather than inline code.
  fenced: boolean;
}

export function codeRanges(text: string): CodeRange[] {
  const ranges: CodeRange[] = [];
  let fenceStart: number | null = null;
  let pos = 0;
  for (const line of text.split("\n")) {
    const end = pos + line.length;
    // As CommonMark: a fence is indented at most three spaces.
    if (/^ {0,3}```/.test(line)) {
      if (fenceStart === null) fenceStart = pos;
      else {
        ranges.push({ from: fenceStart, to: end, fenced: true });
        fenceStart = null;
      }
    } else if (fenceStart === null) {
      for (const m of line.matchAll(/`[^`\n]+`/g)) {
        ranges.push({ from: pos + m.index!, to: pos + m.index! + m[0].length, fenced: false });
      }
    }
    pos = end + 1;
  }
  if (fenceStart !== null) ranges.push({ from: fenceStart, to: text.length, fenced: true });
  return ranges;
}

export function mathSpans(text: string): MathSpan[] {
  const code = codeRanges(text);
  const inCode = (from: number, to: number) => code.some((c) => from < c.to && to > c.from);
  const spans: MathSpan[] = [];
  for (const m of text.matchAll(/\$\$([\s\S]+?)\$\$/g)) {
    const from = m.index!;
    const to = from + m[0].length;
    if (!inCode(from, to) && m[1].trim()) spans.push({ from, to, tex: m[1].trim(), display: true });
  }
  const inDisplay = (i: number) => spans.some((s) => i >= s.from && i < s.to);
  for (const m of text.matchAll(/(?<![\\$])\$(?![\s$])([^$\n]*?[^\s\\$])\$(?!\$)/g)) {
    const from = m.index!;
    const to = from + m[0].length;
    if (!inCode(from, to) && !inDisplay(from) && !inDisplay(to - 1)) spans.push({ from, to, tex: m[1], display: false });
  }
  return spans.sort((a, b) => a.from - b.from);
}
