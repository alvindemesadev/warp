import { describe, it, expect } from "vitest";
import { isSpecialPath, normalizeThrottleInput, normalizeWorkersInput } from "./transfer";

describe("isSpecialPath", () => {
  it("detects OneDrive and network", () => {
    expect(isSpecialPath("C:\\Users\\Alvin\\OneDrive\\Docs")).toBeTruthy();
    expect(isSpecialPath("\\\\SERVER\\share\\folder")).toBeTruthy();
    expect(isSpecialPath("C:\\normal\\path")).toBeNull();
  });
  it("case insensitive", () => {
    expect(isSpecialPath("C:\\ONEDRIVE\\x")).toBeTruthy();
  });
});

describe("normalizeThrottleInput", () => {
  it("clamps", () => {
    expect(normalizeThrottleInput(50)).toBe(50);
    expect(normalizeThrottleInput(0)).toBe(0);
    expect(normalizeThrottleInput(-10)).toBe(0);
    expect(normalizeThrottleInput(999)).toBe(500);
    expect(normalizeThrottleInput(NaN)).toBe(50);
    expect(normalizeThrottleInput(12.7)).toBe(13);
  });
});

describe("normalizeWorkersInput", () => {
  it("keeps 0 as Auto", () => {
    expect(normalizeWorkersInput(0)).toBe(0);
    expect(normalizeWorkersInput(-5)).toBe(0);
    expect(normalizeWorkersInput(NaN)).toBe(0);
  });
  it("clamps explicit counts to 2..8 and rounds", () => {
    expect(normalizeWorkersInput(2)).toBe(2);
    expect(normalizeWorkersInput(4.6)).toBe(5);
    expect(normalizeWorkersInput(1)).toBe(2);
    expect(normalizeWorkersInput(99)).toBe(8);
  });
});
