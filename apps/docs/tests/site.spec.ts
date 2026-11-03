import { expect, test } from "@playwright/test";

for (const route of ["/", "/docs/", "/docs/quickstart/", "/docs/cli/", "/docs/layout/", "/docs/architecture/", "/docs/bindings/", "/playground/"]) {
  test(`renders ${route} without errors or horizontal overflow`, async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(route);
    await expect(page.locator("main h1")).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(errors).toEqual([]);
  });
}

test("renders by default: WASM draws the diagram without any click", async ({ page }) => {
  await page.goto("/playground/");
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
  await expect(page.locator(".diagram-meta")).toContainText("provided DI");
  await expect(page.locator(".diagram-meta")).toContainText("324 × 100");
});

test("edits re-render live: a broken reference surfaces diagnostics without clicking Render", async ({ page }) => {
  await page.goto("/playground/");
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
  await page.getByRole("button", { name: "Introduce an error" }).click();
  await expect(page.locator(".diagnostic")).not.toHaveCount(0);
  await expect(page.locator(".diagnostic-summary")).toContainText("1 errors");
  await page.getByRole("button", { name: "Reset example" }).click();
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
  await page.getByRole("tab", { name: "Diagnostics" }).click();
  await expect(page.getByText("All checks passed")).toBeVisible();
  await expect(page.locator(".result-badge")).toContainText("VALID");
});

test("switching to the auto-layout example renders generated geometry automatically", async ({ page }) => {
  await page.goto("/playground/");
  await page.getByLabel("Choose example").selectOption({ value: "layout" });
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
  await expect(page.locator(".diagram-meta")).toContainText("auto-layout");
  await page.getByRole("tab", { name: "Generated DI" }).click();
  await expect(page.locator(".query-table")).toBeVisible();
  await expect(page.locator(".query-table")).toContainText("Start_1");
});

test("malformed XML surfaces the parse error live and recovers on reset", async ({ page }) => {
  await page.goto("/playground/");
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
  await page.locator(".cm-content").click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("<bpmn:definitions");
  await expect(page.getByText("Could not render this model")).toBeVisible({ timeout: 25_000 });
  await page.getByRole("button", { name: "Reset example" }).click();
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
});

test("render mode switches re-render without a click", async ({ page }) => {
  await page.goto("/playground/");
  await expect(page.locator(".diagram-canvas svg")).toBeVisible({ timeout: 25_000 });
  await page.getByLabel("Render mode").selectOption({ value: "layout" });
  await expect(page.locator(".diagram-meta")).toContainText("auto-layout", { timeout: 25_000 });
  await page.getByRole("tab", { name: "Generated DI" }).click();
  await expect(page.locator(".query-table")).toContainText("StartEvent_1");
});

test("day and night themes persist and produce screenshots", async ({ page }, testInfo) => {
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await page.goto("/");
  await page.evaluate(() => document.fonts.ready);
  await expect(page.locator("html")).toHaveClass(/light/);
  const toolbar = await page.locator(".editor-toolbar").boundingBox();
  const editor = await page.locator(".editor-area:visible").boundingBox();
  const outputBar = await page.locator(".output-bar").boundingBox();
  expect(toolbar && editor && editor.y >= toolbar.y + toolbar.height - 1).toBeTruthy();
  expect(editor && outputBar && outputBar.y >= editor.y + editor.height - 1).toBeTruthy();
  await page.screenshot({ path: testInfo.outputPath("home-day.png"), fullPage: true });
  await page.getByRole("button", { name: "Toggle day / night theme" }).click();
  await expect(page.locator("html")).toHaveClass(/dark/);
  await page.reload();
  await expect(page.locator("html")).toHaveClass(/dark/);
  await page.screenshot({ path: testInfo.outputPath("home-night.png"), fullPage: true });
});
