import { test, expect } from '@playwright/test';

test('Debug Default preset - ParentComponent', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  // Click CF tab
  await page.click('text=CF');
  await page.waitForTimeout(2000);

  // Click on ParentComponent.vue to view it
  await page.click('text=ParentComponent.vue');
  await page.waitForTimeout(1000);

  // Take screenshot
  await page.screenshot({ path: 'test-results/default-parent.png', fullPage: true });

  // Get diagnostics
  const diagPanel = page.locator('.diagnostics-pane');
  const diagText = await diagPanel.first().textContent();
  console.log('=== DIAGNOSTICS ===');
  console.log(diagText);

  expect(true).toBeTruthy();
});

test('Debug Reactivity Loss preset - ChildComponent', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  // Click CF tab
  await page.click('text=CF');
  await page.waitForTimeout(1000);

  // Click Reactivity Loss preset
  await page.click('text=Reactivity Loss');
  await page.waitForTimeout(2000);

  // Click on ChildComponent.vue to view it
  await page.click('text=ChildComponent.vue');
  await page.waitForTimeout(1000);

  // Take screenshot
  await page.screenshot({ path: 'test-results/reactivity-loss-child.png', fullPage: true });

  // Get diagnostics
  const diagPanel = page.locator('.diagnostics-pane');
  const diagText = await diagPanel.first().textContent();
  console.log('=== DIAGNOSTICS ===');
  console.log(diagText);

  expect(true).toBeTruthy();
});
