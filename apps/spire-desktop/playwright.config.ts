import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";

const PORT = 4173;
const BASE_URL = `http://127.0.0.1:${PORT}`;

function resolveSystemChromiumExecutable(): string | undefined {
  const candidates = [
    process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE,
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
    "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  ].filter((value): value is string => Boolean(value));

  return candidates.find((path) => fs.existsSync(path));
}

const systemChromiumExecutable = resolveSystemChromiumExecutable();

export default defineConfig({
  testDir: "./e2e/specs",
  timeout: 90_000,
  expect: {
    timeout: 15_000,
  },
  fullyParallel: false,
  retries: 0,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "off",
    viewport: { width: 1600, height: 1000 },
  },
  webServer: {
    command: `npm run dev:web -- --host 127.0.0.1 --port ${PORT}`,
    port: PORT,
    timeout: 120_000,
    reuseExistingServer: true,
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        channel: systemChromiumExecutable ? undefined : "chrome",
        launchOptions: systemChromiumExecutable
          ? { executablePath: systemChromiumExecutable }
          : undefined,
      },
    },
  ],
});
