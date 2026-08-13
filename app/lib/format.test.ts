// app/lib/format.test.ts
import { describe, expect, it } from "vitest";
import { preview, firstLine, formatWhen } from "./format";

describe("format helpers", () => {
  it("collapses whitespace and truncates with ellipsis", () => {
    const body = "word ".repeat(40).trim();
    const p = preview(body, 80);
    expect(p.endsWith("…")).toBe(true);
    expect(p.length).toBeLessThanOrEqual(81);
  });

  it("keeps short strings intact", () => {
    expect(preview("  hello   world  ")).toBe("hello world");
  });

  it("extracts the first non-empty line", () => {
    expect(firstLine("\n\nBuy milk\nand eggs")).toBe("Buy milk");
    expect(firstLine("   ")).toBe("");
  });

  it("formats recent timestamps relatively", () => {
    expect(formatWhen(new Date().toISOString())).toBe("just now");
    expect(formatWhen("garbage")).toBe("");
  });
});
