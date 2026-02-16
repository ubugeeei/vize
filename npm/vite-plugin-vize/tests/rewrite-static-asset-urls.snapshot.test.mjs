import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { __internal } from "../dist/index.js";

function normalizeNewlines(text) {
  return text.replace(/\r\n/g, "\n").trimEnd();
}

const input = [
  'const _hoisted_1 = { src: "@/assets/images/logo.svg", alt: "" };',
  'const _hoisted_2 = { "src": \'@/assets/icons/help.svg\' };',
  'const _hoisted_3 = { src: "/assets/local.svg" };',
].join("\n");

const output = __internal.rewriteStaticAssetUrls(input, [
  {
    fromPrefix: "@/",
    toPrefix: "/@fs/unused/",
  },
]);

const testDir = path.dirname(fileURLToPath(import.meta.url));
const snapshotPath = path.join(testDir, "snapshots", "rewrite-static-asset-urls.snap");
const expected = fs.readFileSync(snapshotPath, "utf8");

assert.strictEqual(
  normalizeNewlines(output),
  normalizeNewlines(expected),
  "Bug-40 snapshot mismatch: static src alias rewrite output changed",
);

console.log("✅ rewriteStaticAssetUrls snapshot test passed");
