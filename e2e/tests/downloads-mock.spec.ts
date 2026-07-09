import { test, expect } from '@playwright/test';
import * as path from 'path';

const FIXTURES = path.resolve(__dirname, '../fixtures');

test.describe('Mock-backed downloads', () => {
  test('uploading the sample NZB completes and lands in history', async ({ page }) => {
    await page.goto('/downloads');

    await page.getByRole('button', { name: /\+ upload nzb/i }).click();
    await page.locator('input[type="file"]').setInputFiles(path.join(FIXTURES, 'sample.nzb'));
    await page.locator('.add-panel').getByRole('button', { name: /upload/i }).click();

    await expect(page.getByText(/added to queue/i)).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: 'History' }).click();
    await expect(page).toHaveURL(/\/downloads\?tab=history/);

    const completedRow = page.locator('tr', { hasText: /sample/i }).first();
    await expect(completedRow).toBeVisible({ timeout: 20000 });
    await expect(completedRow.locator('.s-ok')).toBeVisible();
  });
});
