import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30000,
  use: {
    baseURL: 'http://localhost:5180/play/',
  },
  webServer: {
    command: 'pnpm dev',
    url: 'http://localhost:5180/play/',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
