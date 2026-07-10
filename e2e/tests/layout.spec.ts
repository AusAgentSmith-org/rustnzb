import { test, expect } from '@playwright/test';

test.describe('12. Layout preferences', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/downloads');
    await page.evaluate(() => localStorage.removeItem('rustnzb.widthMode'));
    await page.reload();
  });

  test('12.1 compact and expanded controls expose their selected state', async ({ page }) => {
    const compact = page.getByRole('button', { name: 'Compact layout' });
    const expanded = page.getByRole('button', { name: 'Expanded layout' });

    await expanded.click();
    await expect(expanded).toHaveAttribute('aria-pressed', 'true');
    await expect(compact).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('body')).toHaveAttribute('data-width-mode', 'expanded');

    await compact.click();
    await expect(compact).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('body')).toHaveAttribute('data-width-mode', 'compact');
  });

  test('12.2 explicit width mode persists across reloads', async ({ page }) => {
    await page.getByRole('button', { name: 'Expanded layout' }).click();
    await page.reload();

    await expect(page.getByRole('button', { name: 'Expanded layout' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(await page.evaluate(() => localStorage.getItem('rustnzb.widthMode'))).toBe('expanded');
  });

  test('12.3 layout controls remain usable at a narrow viewport', async ({ page }) => {
    await page.setViewportSize({ width: 800, height: 700 });
    await expect(page.getByRole('button', { name: 'Compact layout' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Expanded layout' })).toBeVisible();
  });
});
