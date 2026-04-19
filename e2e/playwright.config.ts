import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  expect: { timeout: 10000 },
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'playwright-report' }]],
  globalSetup: './global-setup.ts',
  globalTeardown: './global-teardown.ts',

  projects: [
    // ── Main: authenticated, seeded data ──────────────────────────────────────
    {
      name: 'main',
      testIgnore: ['**/first-boot.spec.ts', '**/auth.spec.ts'],
      use: {
        ...devices['Desktop Chrome'],
        baseURL: 'http://localhost:9190',
        storageState: 'auth-state.json',
        trace: 'on-first-retry',
        screenshot: 'only-on-failure',
      },
    },
    // ── Fresh: no credentials (first-boot + auth flow tests) ─────────────────
    {
      name: 'fresh',
      testMatch: ['**/first-boot.spec.ts', '**/auth.spec.ts'],
      use: {
        ...devices['Desktop Chrome'],
        baseURL: 'http://localhost:9191',
        // No storageState — tests manage their own tokens
        trace: 'on-first-retry',
        screenshot: 'only-on-failure',
      },
    },
  ],
});
