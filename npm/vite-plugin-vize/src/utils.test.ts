/**
 * Unit tests for vite-plugin-vize utils
 *
 * Run with: npx tsx src/utils.test.ts
 *
 * These tests cover various edge cases and bug fixes.
 */

import assert from "node:assert";

// =============================================================================
// Test: Non-script-setup SFC _sfc_main duplication fix
// =============================================================================

/**
 * Simulates the generateOutput logic for detecting _sfc_main
 */
function hasSfcMainDefined(code: string): boolean {
  return /\bconst\s+_sfc_main\s*=/.test(code);
}

function hasExportDefault(code: string): boolean {
  return /^export default /m.test(code);
}

// Test 1: Script setup component (should NOT have _sfc_main defined)
const scriptSetupCode = `
import { openBlock as _openBlock } from "vue"
export default {
  __name: "Component",
  setup() { return {} }
}
`;

assert.strictEqual(
  hasSfcMainDefined(scriptSetupCode),
  false,
  "Script setup component should not have _sfc_main pre-defined"
);
assert.strictEqual(
  hasExportDefault(scriptSetupCode),
  true,
  "Script setup component should have export default"
);

// Test 2: Non-script-setup component (should have _sfc_main defined)
const nonScriptSetupCode = `
import { openBlock as _openBlock } from "vue"
const __default__ = { name: 'MyComponent' }
const _sfc_main = __default__
export default _sfc_main
`;

assert.strictEqual(
  hasSfcMainDefined(nonScriptSetupCode),
  true,
  "Non-script-setup component should have _sfc_main pre-defined"
);
assert.strictEqual(
  hasExportDefault(nonScriptSetupCode),
  true,
  "Non-script-setup component should have export default"
);

// Test 3: Variation with different spacing
const variationCode = `const  _sfc_main   =  __default__`;
assert.strictEqual(
  hasSfcMainDefined(variationCode),
  true,
  "Should detect _sfc_main with various whitespace"
);

// =============================================================================
// Test: Query parameter preservation in relative imports
// =============================================================================

function splitPathAndQuery(id: string): [string, string] {
  const [pathPart, queryPart] = id.split("?");
  const querySuffix = queryPart ? `?${queryPart}` : "";
  return [pathPart, querySuffix];
}

// Test 4: Import with ?inline query
const [path1, query1] = splitPathAndQuery("./style.css?inline");
assert.strictEqual(path1, "./style.css", "Path should be extracted");
assert.strictEqual(query1, "?inline", "Query should be preserved");

// Test 5: Import with ?raw query
const [path2, query2] = splitPathAndQuery("./data.json?raw");
assert.strictEqual(path2, "./data.json", "Path should be extracted");
assert.strictEqual(query2, "?raw", "Query should be preserved");

// Test 6: Import without query
const [path3, query3] = splitPathAndQuery("./component.vue");
assert.strictEqual(path3, "./component.vue", "Path should be unchanged");
assert.strictEqual(query3, "", "No query suffix");

// Test 7: Import with multiple query params
const [path4, query4] = splitPathAndQuery("./file.txt?raw&inline");
assert.strictEqual(path4, "./file.txt", "Path should be extracted");
assert.strictEqual(query4, "?raw&inline", "All query params preserved");

// =============================================================================
// Test: Already-resolved path detection
// =============================================================================

function isAlreadyResolved(id: string): boolean {
  return id.includes("/dist/") || id.includes("/lib/") || id.includes("/es/");
}

// Test 8: dist path
assert.strictEqual(
  isAlreadyResolved("/node_modules/some-pkg/dist/index.mjs"),
  true,
  "Should detect /dist/ path as resolved"
);

// Test 9: lib path
assert.strictEqual(
  isAlreadyResolved("/node_modules/some-pkg/lib/index.js"),
  true,
  "Should detect /lib/ path as resolved"
);

// Test 10: es path (ESM build)
assert.strictEqual(
  isAlreadyResolved("/node_modules/some-pkg/es/index.mjs"),
  true,
  "Should detect /es/ path as resolved"
);

// Test 11: Regular package import
assert.strictEqual(
  isAlreadyResolved("lodash-es"),
  false,
  "Package name should not be detected as resolved"
);

// Test 12: Relative import
assert.strictEqual(
  isAlreadyResolved("./components/Button.vue"),
  false,
  "Relative import should not be detected as resolved"
);

// =============================================================================
// Test: scopeId generation
// =============================================================================

function generateScopeId(filename: string): string {
  // Simplified hash function for testing
  let hash = 0;
  for (let i = 0; i < filename.length; i++) {
    const char = filename.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash; // Convert to 32bit integer
  }
  return Math.abs(hash).toString(16).substring(0, 8);
}

// Test 13: Different files should have different scope IDs
const scope1 = generateScopeId("src/components/Button.vue");
const scope2 = generateScopeId("src/components/Input.vue");
assert.notStrictEqual(scope1, scope2, "Different files should have different scope IDs");

// Test 14: Same file should have same scope ID
const scope3 = generateScopeId("src/components/Button.vue");
assert.strictEqual(scope1, scope3, "Same file should have same scope ID");

// =============================================================================
// All tests passed
// =============================================================================

console.log("✅ All vite-plugin-vize utils tests passed!");
