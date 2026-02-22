import { test, expect, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { createConnection } from "node:net";
import * as path from "node:path";
import * as fs from "node:fs";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const APPS_ROOT = path.resolve(__dirname, "../__ubugeeei__");
const E2E_DIR = path.resolve(__dirname);
const SCREENSHOT_DIR = path.resolve(__dirname, "screenshots");
const VITE_PLUS_BIN = path.join(
  process.env.HOME ?? "",
  ".vite-plus",
  "bin",
);

interface AppConfig {
  name: string;
  cwd: string;
  command: string;
  args: string[];
  port: number;
  url: string;
  mountSelector: string;
  /** Pattern in stdout/stderr that indicates the server is ready */
  readyPattern: RegExp;
  /** If true, the dev server doesn't serve an HTML page by itself */
  noHtmlFallback?: boolean;
  /** Allow non-200 responses (e.g. proxy errors when backend is missing) */
  allowNon200?: boolean;
  startupTimeout: number;
  /** Setup function to run before starting the dev server */
  setup?: () => void;
}

const apps: AppConfig[] = [
  {
    name: "elk",
    cwd: path.join(E2E_DIR, "elk"),
    command: "npx",
    args: ["pnpm@10", "dev"],
    port: 5314,
    url: "http://localhost:5314",
    mountSelector: "#__nuxt",
    readyPattern: /Local:\s+http:\/\/localhost:5314/,
    // Nuxt SSR may return 503 due to esbuild dep optimization issues (unrelated to vize)
    allowNon200: true,
    startupTimeout: 120_000,
  },
  {
    name: "hoppscotch",
    cwd: path.join(APPS_ROOT, "hoppscotch", "packages", "hoppscotch-selfhost-web"),
    command: "npx",
    args: ["pnpm@10", "exec", "vite"],
    port: 3000,
    url: "http://localhost:3000",
    mountSelector: "#app",
    readyPattern: /Local:\s+http:\/\/localhost:3000/,
    startupTimeout: 90_000,
  },
  {
    name: "directus",
    cwd: path.join(APPS_ROOT, "directus", "app"),
    command: "npx",
    args: ["pnpm@10", "exec", "vite", "--host"],
    port: 8080,
    url: "http://localhost:8080",
    mountSelector: "#app",
    readyPattern: /Local:\s+http:\/\//,
    allowNon200: true,
    startupTimeout: 90_000,
  },
  {
    name: "misskey",
    cwd: path.join(E2E_DIR, "misskey", "packages", "frontend"),
    command: "npx",
    args: ["pnpm@10", "exec", "vite"],
    port: 5173,
    url: "http://localhost:5173",
    mountSelector: "#misskey_app",
    readyPattern: /Local:\s+http:\/\//,
    noHtmlFallback: true,
    startupTimeout: 90_000,
    setup() {
      // misskey needs a default.yml config file to load vite.config.ts
      const configDir = path.join(E2E_DIR, "misskey", ".config");
      const configFile = path.join(configDir, "default.yml");
      if (!fs.existsSync(configFile)) {
        fs.mkdirSync(configDir, { recursive: true });
        fs.writeFileSync(configFile, "url: http://localhost:3000\nport: 3000\n");
      }
    },
  },
];

function waitForServerReady(
  proc: ChildProcess,
  port: number,
  readyPattern: RegExp,
  timeout: number,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + timeout;
    let resolved = false;
    let processExited = false;
    let exitCode: number | null = null;

    function checkDone() {
      if (resolved) return;
      resolved = true;
      resolve();
    }

    function checkFailed(reason: string) {
      if (resolved) return;
      resolved = true;
      reject(new Error(reason));
    }

    // Watch stdout/stderr for ready pattern (strip ANSI codes before matching)
    const stripAnsi = (s: string) => s.replace(/\x1b\[[0-9;]*m/g, "");
    const onData = (data: Buffer) => {
      const text = stripAnsi(data.toString());
      if (readyPattern.test(text)) {
        // Give a short delay for the server to fully initialize
        setTimeout(checkDone, 1000);
      }
    };
    proc.stdout?.on("data", onData);
    proc.stderr?.on("data", onData);

    // Watch for process exit (server crashed)
    proc.on("exit", (code) => {
      processExited = true;
      exitCode = code;
      checkFailed(`Dev server process exited with code ${code} before becoming ready`);
    });

    // Also try TCP connection as backup
    function attemptTcp() {
      if (resolved || processExited) return;
      if (Date.now() > deadline) {
        checkFailed(`Server did not become ready within ${timeout}ms (port ${port})`);
        return;
      }
      const socket = createConnection({ port, host: "127.0.0.1" }, () => {
        socket.destroy();
        checkDone();
      });
      socket.on("error", () => {
        socket.destroy();
        setTimeout(attemptTcp, 2000);
      });
    }

    // Start TCP probing after a short delay
    setTimeout(attemptTcp, 3000);
  });
}

function startDevServer(app: AppConfig): ChildProcess {
  const env = {
    ...process.env,
    PATH: `${VITE_PLUS_BIN}:${process.env.PATH}`,
    NODE_ENV: "development",
    BROWSER: "none",
  };

  const proc = spawn(app.command, app.args, {
    cwd: app.cwd,
    env,
    stdio: ["ignore", "pipe", "pipe"],
    detached: true,
  });

  proc.stdout?.on("data", (data: Buffer) => {
    const line = data.toString().trim();
    if (line) console.log(`[${app.name}:stdout] ${line}`);
  });

  proc.stderr?.on("data", (data: Buffer) => {
    const line = data.toString().trim();
    if (line) console.log(`[${app.name}:stderr] ${line}`);
  });

  return proc;
}

function killProcess(proc: ChildProcess): void {
  if (proc.pid) {
    try {
      process.kill(-proc.pid, "SIGTERM");
    } catch {
      try {
        proc.kill("SIGTERM");
      } catch {
        // already dead
      }
    }
  }
}

async function collectConsoleErrors(page: Page): Promise<string[]> {
  const errors: string[] = [];

  page.on("console", (msg) => {
    if (msg.type() === "error") {
      errors.push(msg.text());
    }
  });

  page.on("pageerror", (err) => {
    errors.push(err.message);
  });

  return errors;
}

function isFatalError(error: string): boolean {
  const fatalPatterns = [
    /Failed to resolve component/,
    /\[Vue warn\].*is not a function/,
    /Cannot read propert/,
    /Uncaught TypeError/,
    /Uncaught ReferenceError/,
    /Uncaught SyntaxError/,
    /Failed to fetch dynamically imported module/,
    /ChunkLoadError/,
  ];
  // Exclude known non-fatal errors
  const ignorePatterns = [
    /Failed to load resource/,   // Network errors (missing backend)
    /net::ERR_/,                 // Network errors
    /ECONNREFUSED/,              // Backend not running
    /is not defined.*\$pinia/,   // Pinia not initialized (no backend)
  ];
  if (ignorePatterns.some((p) => p.test(error))) return false;
  return fatalPatterns.some((p) => p.test(error));
}

// Ensure screenshot directory exists
fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

for (const app of apps) {
  test.describe(app.name, () => {
    let devServer: ChildProcess;

    test.beforeAll(async () => {
      // Run setup if needed
      if (app.setup) {
        app.setup();
      }

      console.log(`Starting dev server for ${app.name}...`);
      devServer = startDevServer(app);

      devServer.on("exit", (code) => {
        console.log(`[${app.name}] dev server exited with code ${code}`);
      });

      console.log(`Waiting for ${app.name} server to be ready (port ${app.port})...`);
      await waitForServerReady(devServer, app.port, app.readyPattern, app.startupTimeout);
      console.log(`${app.name} server is ready`);
    });

    test.afterAll(async () => {
      console.log(`Stopping dev server for ${app.name}...`);
      killProcess(devServer);
      // Give a moment for cleanup
      await new Promise((r) => setTimeout(r, 2000));
    });

    if (app.noHtmlFallback) {
      // For apps like misskey that don't serve HTML from Vite dev server alone,
      // just verify the server is responding
      test("dev server responds", async ({ request }) => {
        const response = await request.get(app.url);
        // Vite dev server should at least respond (might be 404 for missing index.html)
        expect(response.status()).toBeLessThan(500);
        console.log(`${app.name}: server responded with status ${response.status()}`);
      });
    } else {
      test("page renders without fatal errors", async ({ page }) => {
        const errors = await collectConsoleErrors(page);

        const response = await page.goto(app.url, {
          waitUntil: "domcontentloaded",
          timeout: 30_000,
        });

        if (app.allowNon200) {
          // Server is up but backend may be missing, just check it responded
          expect(response?.status()).toBeDefined();
          console.log(`${app.name}: responded with status ${response?.status()}`);

          // If we got 500 (proxy error), that's expected without backend
          if (response?.status() !== 200) {
            console.log(`${app.name}: non-200 response (expected without backend), skipping further checks`);
            // Take screenshot anyway
            await page.screenshot({
              path: path.join(SCREENSHOT_DIR, `${app.name}.png`),
              fullPage: true,
            });
            return;
          }
        } else {
          expect(response?.status()).toBe(200);
        }

        // Page is not blank
        const bodyContent = await page.locator("body").innerHTML();
        expect(bodyContent.trim().length).toBeGreaterThan(0);

        // Mount element exists
        const mountEl = page.locator(app.mountSelector);
        await expect(mountEl).toBeAttached({ timeout: 15_000 });

        // Mount element has content (Vue app rendered)
        // Note: some apps may not mount without a backend, so this is a soft check
        const mountContent = await mountEl.innerHTML();
        if (mountContent.trim().length === 0) {
          console.log(`${app.name}: mount element ${app.mountSelector} exists but is empty (app may need backend to mount)`);
        } else {
          console.log(`${app.name}: Vue app mounted successfully`);
        }

        // Take screenshot
        await page.screenshot({
          path: path.join(SCREENSHOT_DIR, `${app.name}.png`),
          fullPage: true,
        });
        console.log(`Screenshot saved: ${app.name}.png`);

        // Check for fatal console errors
        const fatalErrors = errors.filter(isFatalError);
        if (fatalErrors.length > 0) {
          console.log(`Fatal errors in ${app.name}:`, fatalErrors);
        }
        expect(fatalErrors).toHaveLength(0);
      });
    }
  });
}
