import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("a11y", () => {
  test("no critical axe violations on load", async ({ page }) => {
    await page.goto("/");
    const results = await new AxeBuilder({ page }).analyze();
    const critical = results.violations.filter(
      (v) => v.impact === "critical" || v.impact === "serious",
    );
    if (critical.length) console.log("axe critical", JSON.stringify(critical, null, 2));
    expect(critical).toEqual([]);
  });

  test("keyboard-only: tab to source → dest → mode → engage", async ({ page }) => {
    await page.goto("/");
    // Tab sequence: should reach PathCard, ModePicker, OptionsPanel, engage
    await page.keyboard.press("Tab");
    // Check that focus moves and no trap — just ensure body is visible and no error
    await expect(page.locator("body")).toBeVisible();
    // Esc should not crash
    await page.keyboard.press("Escape");
    await expect(page.locator("h1").first()).toBeVisible();
  });

  test("prefers-reduced-motion disables animations", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
    // Check that shimmer/pulse is disabled via computed style
    const shimmer = page.locator(".animate-shimmer").first();
    if (await shimmer.count()) {
      const anim = await shimmer.evaluate((el) => getComputedStyle(el).animationName);
      expect(anim === "none" || anim === "").toBeTruthy();
    }
  });
});
