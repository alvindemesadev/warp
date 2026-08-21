import { describe, it, expect } from "vitest";
import { isSpecialPath, normalizeThrottleInput, isPresetThrottle, THROTTLE_OPTIONS } from "./transfer";

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

describe("isPresetThrottle", () => {
  it("matches preset values", () => {
    for (const o of THROTTLE_OPTIONS) expect(isPresetThrottle(o.value)).toBe(true);
    expect(isPresetThrottle(50)).toBe(false);
    expect(isPresetThrottle(7)).toBe(false);
  });
});
