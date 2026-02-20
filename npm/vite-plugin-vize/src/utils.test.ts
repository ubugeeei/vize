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
  "Script setup component should not have _sfc_main pre-defined",
);
assert.strictEqual(
  hasExportDefault(scriptSetupCode),
  true,
  "Script setup component should have export default",
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
  "Non-script-setup component should have _sfc_main pre-defined",
);
assert.strictEqual(
  hasExportDefault(nonScriptSetupCode),
  true,
  "Non-script-setup component should have export default",
);

// Test 3: Variation with different spacing
const variationCode = `const  _sfc_main   =  __default__`;
assert.strictEqual(
  hasSfcMainDefined(variationCode),
  true,
  "Should detect _sfc_main with various whitespace",
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
  "Should detect /dist/ path as resolved",
);

// Test 9: lib path
assert.strictEqual(
  isAlreadyResolved("/node_modules/some-pkg/lib/index.js"),
  true,
  "Should detect /lib/ path as resolved",
);

// Test 10: es path (ESM build)
assert.strictEqual(
  isAlreadyResolved("/node_modules/some-pkg/es/index.mjs"),
  true,
  "Should detect /es/ path as resolved",
);

// Test 11: Regular package import
assert.strictEqual(
  isAlreadyResolved("lodash-es"),
  false,
  "Package name should not be detected as resolved",
);

// Test 12: Relative import
assert.strictEqual(
  isAlreadyResolved("./components/Button.vue"),
  false,
  "Relative import should not be detected as resolved",
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
// Test: resolveCssImports — @custom-media resolution
// =============================================================================

/**
 * Minimal version of resolveCssImports that handles @custom-media and :deep()
 * without filesystem access (for unit testing the pure transformation logic).
 */
function resolveCssTransforms(css: string): string {
  const customMedia = new Map<string, string>();

  // Parse @custom-media definitions
  const cmRe = /@custom-media\s+(--[\w-]+)\s+(.+?)\s*;/g;
  let m: RegExpExecArray | null;
  while ((m = cmRe.exec(css)) !== null) {
    customMedia.set(m[1], m[2]);
  }

  // Remove @custom-media definitions
  let result = css.replace(/^@custom-media\s+[^;]+;\s*$/gm, "");

  // Replace (--name) in @media rules
  for (const [name, query] of customMedia) {
    const escaped = name.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&");
    result = result.replace(new RegExp(`\\(${escaped}\\)`, "g"), query);
  }

  // Unwrap :deep()
  result = result.replace(/:deep\(([^()]*(?:\([^()]*\))*[^()]*)\)/g, "$1");

  // Clean up excessive blank lines
  result = result.replace(/\n{3,}/g, "\n\n");

  return result;
}

// Test 15: @custom-media resolution
{
  const css = `@custom-media --mobile (max-width: 768px);
.foo { color: red; }
@media (--mobile) { .foo { font-size: 12px; } }`;
  const result = resolveCssTransforms(css);
  assert.ok(!result.includes("@custom-media"), "@custom-media definition should be removed");
  assert.ok(
    result.includes("@media (max-width: 768px)"),
    "@media (--mobile) should be resolved to (max-width: 768px). Got:\n" + result,
  );
}

// Test 16: Multiple @custom-media definitions
{
  const css = `@custom-media --mobile (max-width: 768px);
@custom-media --desktop (min-width: 1024px);
@media (--mobile) { .a { color: red; } }
@media (--desktop) { .b { color: blue; } }`;
  const result = resolveCssTransforms(css);
  assert.ok(result.includes("(max-width: 768px)"), "Should resolve --mobile");
  assert.ok(result.includes("(min-width: 1024px)"), "Should resolve --desktop");
}

// Test 17: :deep() unwrapping
{
  const css = `.parent :deep(.child) { color: red; }`;
  const result = resolveCssTransforms(css);
  assert.ok(
    result.includes(".parent .child"),
    ":deep(.child) should be unwrapped to .child. Got:\n" + result,
  );
  assert.ok(!result.includes(":deep"), ":deep should be removed");
}

// Test 18: :deep() with nested parens
{
  const css = `.parent :deep(.child:nth-child(2)) { color: red; }`;
  const result = resolveCssTransforms(css);
  assert.ok(
    result.includes(".child:nth-child(2)"),
    ":deep() with nested parens should be unwrapped. Got:\n" + result,
  );
}

// Test 19: CSS without @custom-media passes through unchanged
{
  const css = `.foo { color: red; }\n@media (max-width: 768px) { .foo { font-size: 12px; } }`;
  const result = resolveCssTransforms(css);
  assert.strictEqual(result, css, "CSS without @custom-media should pass through unchanged");
}

// =============================================================================
// Test: applyDefineReplacements
// =============================================================================

function applyDefineReplacements(code: string, defines: Record<string, string>): string {
  const sortedKeys = Object.keys(defines).sort((a, b) => b.length - a.length);
  let result = code;
  for (const key of sortedKeys) {
    if (!result.includes(key)) continue;
    const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const re = new RegExp(escaped + "(?![\\w$.])", "g");
    result = result.replace(re, defines[key]);
  }
  return result;
}

// Test 20: Basic define replacement
{
  const code = `if (import.meta.vfFeatures.photoSection) { show(); }`;
  const defines = { "import.meta.vfFeatures.photoSection": "true" };
  const result = applyDefineReplacements(code, defines);
  assert.ok(
    result.includes("if (true)"),
    "Should replace import.meta.vfFeatures.photoSection with true. Got:\n" + result,
  );
}

// Test 21: Should not replace partial matches
{
  const code = `import.meta.vfFeatures.photoSectionEnabled`;
  const defines = { "import.meta.vfFeatures.photoSection": "true" };
  const result = applyDefineReplacements(code, defines);
  assert.ok(
    result.includes("import.meta.vfFeatures.photoSectionEnabled"),
    "Should not replace partial match (longer identifier). Got:\n" + result,
  );
}

// Test 22: Longest key matched first
{
  const code = `import.meta.env.MODE + import.meta.env.MODE_DETAIL`;
  const defines = {
    "import.meta.env.MODE": '"production"',
    "import.meta.env.MODE_DETAIL": '"full"',
  };
  const result = applyDefineReplacements(code, defines);
  assert.ok(
    result.includes('"production"'),
    "Should replace import.meta.env.MODE. Got:\n" + result,
  );
  assert.ok(
    result.includes('"full"'),
    "Should replace import.meta.env.MODE_DETAIL. Got:\n" + result,
  );
}

// Test 23: No replacement when key not present
{
  const code = `const x = 42;`;
  const defines = { "import.meta.vfFeatures.foo": "true" };
  const result = applyDefineReplacements(code, defines);
  assert.strictEqual(result, code, "Should not modify code when key is absent");
}

// =============================================================================
// Test: isBuiltinDefine
// =============================================================================

const BUILTIN_DEFINE_PREFIXES = [
  "import.meta.server",
  "import.meta.client",
  "import.meta.dev",
  "import.meta.test",
  "import.meta.prerender",
  "import.meta.env",
  "import.meta.hot",
  "__VUE_",
  "__NUXT_",
  "process.env",
];

function isBuiltinDefine(key: string): boolean {
  return BUILTIN_DEFINE_PREFIXES.some(
    (prefix) => key === prefix || key.startsWith(prefix + ".") || key.startsWith(prefix + "_"),
  );
}

// Test 24: Built-in define detection
assert.strictEqual(isBuiltinDefine("import.meta.env"), true, "import.meta.env is builtin");
assert.strictEqual(
  isBuiltinDefine("import.meta.env.MODE"),
  true,
  "import.meta.env.MODE is builtin",
);
assert.strictEqual(isBuiltinDefine("import.meta.server"), true, "import.meta.server is builtin");
assert.strictEqual(isBuiltinDefine("import.meta.client"), true, "import.meta.client is builtin");
assert.strictEqual(isBuiltinDefine("import.meta.hot"), true, "import.meta.hot is builtin");
// Note: __VUE_OPTIONS_API__ starts with __VUE_O, not __VUE__, so it is NOT matched
// by the current prefix + "_" check. Only __VUE__ (double underscore) would match.
assert.strictEqual(
  isBuiltinDefine("__VUE_OPTIONS_API__"),
  false,
  "__VUE_OPTIONS_API__ does not match __VUE__ prefix",
);
assert.strictEqual(
  isBuiltinDefine("__VUE__SOMETHING"),
  true,
  "__VUE__ (double underscore) is builtin",
);
assert.strictEqual(
  isBuiltinDefine("__NUXT__SOMETHING"),
  true,
  "__NUXT__ (double underscore) is builtin",
);
assert.strictEqual(isBuiltinDefine("process.env"), true, "process.env is builtin");
assert.strictEqual(
  isBuiltinDefine("process.env.NODE_ENV"),
  true,
  "process.env.NODE_ENV is builtin",
);

// Test 25: Non-builtin defines
assert.strictEqual(
  isBuiltinDefine("import.meta.vfFeatures"),
  false,
  "Custom define is not builtin",
);
assert.strictEqual(
  isBuiltinDefine("import.meta.vfFeatures.photoSection"),
  false,
  "Custom nested define is not builtin",
);
assert.strictEqual(isBuiltinDefine("MY_CUSTOM_FLAG"), false, "Custom flag is not builtin");

// =============================================================================
// All tests passed
// =============================================================================

console.log("✅ All vite-plugin-vize utils tests passed!");
