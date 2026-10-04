import { describe, expect, it } from "vitest";
import { mathSpans } from "./math-spans";

const tex = (text: string) => mathSpans(text).map((s) => [s.tex, s.display]);

describe("mathSpans", () => {
  it("finds inline and display math", () => {
    expect(tex("so $G_t$ and\n$$v = 1$$\nok")).toEqual([["G_t", false], ["v = 1", true]]);
  });

  it("lets display math span lines", () => {
    expect(tex("$$\na = b\n$$")).toEqual([["a = b", true]]);
  });

  it("leaves prices and escaped dollars as prose", () => {
    expect(tex("costs $5 and $10")).toEqual([]);
    expect(tex("a \\$x$ b")).toEqual([]);
  });

  it("ignores dollars in code", () => {
    expect(tex("`$x$` and\n```\n$y$\n```\n$z$")).toEqual([["z", false]]);
  });

  it("does not treat an unclosed dollar as math", () => {
    expect(tex("half $typed")).toEqual([]);
  });
});
