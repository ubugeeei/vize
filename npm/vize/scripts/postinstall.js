#!/usr/bin/env node

import { createWriteStream, chmodSync, existsSync, mkdirSync, unlinkSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import { pipeline } from "stream/promises";
import { createGunzip } from "zlib";

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageJson = await import("../package.json", { with: { type: "json" } });
const version = packageJson.default.version;

const REPO = "vizejs/vize";

const PLATFORMS = {
  "darwin-x64": {
    target: "x86_64-apple-darwin",
    filename: "vize-x86_64-apple-darwin.tar.gz",
  },
  "darwin-arm64": {
    target: "aarch64-apple-darwin",
    filename: "vize-aarch64-apple-darwin.tar.gz",
  },
  "linux-x64": {
    target: "x86_64-unknown-linux-gnu",
    filename: "vize-x86_64-unknown-linux-gnu.tar.gz",
  },
  "linux-arm64": {
    target: "aarch64-unknown-linux-gnu",
    filename: "vize-aarch64-unknown-linux-gnu.tar.gz",
  },
  "win32-x64": {
    target: "x86_64-pc-windows-msvc",
    filename: "vize-x86_64-pc-windows-msvc.zip",
  },
  "win32-arm64": {
    target: "aarch64-pc-windows-msvc",
    filename: "vize-aarch64-pc-windows-msvc.zip",
  },
};

async function main() {
  // Skip in CI environment - binary is built separately and not available during pnpm install
  if (process.env.CI) {
    console.log("CI environment detected, skipping binary download.");
    return;
  }

  const platform = `${process.platform}-${process.arch}`;
  const config = PLATFORMS[platform];

  if (!config) {
    console.error(`Unsupported platform: ${platform}`);
    console.error(`Supported platforms: ${Object.keys(PLATFORMS).join(", ")}`);
    console.error(
      "\nYou can install the CLI from source using: cargo install vize"
    );
    process.exit(1);
  }

  const binDir = join(__dirname, "..", "bin");
  const binaryName = process.platform === "win32" ? "vize.exe" : "vize";
  const binaryPath = join(binDir, binaryName);

  // Skip if binary already exists (for local development)
  if (existsSync(binaryPath)) {
    console.log("Vize binary already exists, skipping download.");
    return;
  }

  if (!existsSync(binDir)) {
    mkdirSync(binDir, { recursive: true });
  }

  const tag = `v${version}`;
  const downloadUrl = `https://github.com/${REPO}/releases/download/${tag}/${config.filename}`;

  console.log(`Downloading Vize ${version} for ${platform}...`);
  console.log(`URL: ${downloadUrl}`);

  try {
    const response = await fetch(downloadUrl);

    if (!response.ok) {
      throw new Error(`Failed to download: ${response.status} ${response.statusText}`);
    }

    const tempFile = join(binDir, config.filename);

    // Download to temp file
    const fileStream = createWriteStream(tempFile);
    await pipeline(response.body, fileStream);

    // Extract based on file type
    if (config.filename.endsWith(".tar.gz")) {
      await extractTarGz(tempFile, binDir, binaryName);
    } else if (config.filename.endsWith(".zip")) {
      await extractZip(tempFile, binDir, binaryName);
    }

    // Clean up temp file
    unlinkSync(tempFile);

    // Make executable on Unix
    if (process.platform !== "win32") {
      chmodSync(binaryPath, 0o755);
    }

    console.log(`Vize ${version} installed successfully!`);
  } catch (error) {
    console.error(`Failed to install Vize: ${error.message}`);
    console.error(
      "\nYou can install the CLI from source using: cargo install vize"
    );
    process.exit(1);
  }
}

async function extractTarGz(archivePath, destDir, binaryName) {
  const { execSync } = await import("child_process");
  execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: "inherit" });

  // The archive contains a 'vize' binary directly
  // If not in place, move it
  const extractedPath = join(destDir, "vize");
  const targetPath = join(destDir, binaryName);
  if (extractedPath !== targetPath && existsSync(extractedPath)) {
    const { renameSync } = await import("fs");
    renameSync(extractedPath, targetPath);
  }
}

async function extractZip(archivePath, destDir, binaryName) {
  const { execSync } = await import("child_process");
  execSync(`unzip -o "${archivePath}" -d "${destDir}"`, { stdio: "inherit" });
}

main();
