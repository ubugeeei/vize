import { strict as assert } from "node:assert";
import { generateOutput } from "./utils.js";
import type { CompiledModule } from "./types.js";

type OutputOpts = Parameters<typeof generateOutput>[1];

function runGenerate(code: string, hasScoped = false, scopeId = "scope") {
  const compiled: CompiledModule = {
    code,
    css: "",
    scopeId,
    hasScoped,
    templateHash: "",
    styleHash: "",
    scriptHash: "",
  };
  const options: OutputOpts = {
    isProduction: false,
    isDev: false,
  };
  return generateOutput(compiled, options);
}

// Should rewrite export default when _sfc_main is not defined
{
  const output = runGenerate("export default { name: 'Comp' }", true, "abc123");
  assert.ok(output.includes("const _sfc_main = { name: 'Comp' }"));
  assert.ok(output.includes("export default _sfc_main"));
  assert.ok(output.includes("_sfc_main.__scopeId = \"data-v-abc123\""));
}

// Should not duplicate _sfc_main when already defined
{
  const output = runGenerate(
    "const _sfc_main = { name: 'Comp' }\nexport default _sfc_main\n",
    false,
    "deadbeef",
  );
  const matches = output.match(/const _sfc_main/g) ?? [];
  assert.equal(matches.length, 1);
}

// Should still append scopeId when _sfc_main already exists
{
  const output = runGenerate(
    "const _sfc_main = { name: 'Comp' }\nexport default _sfc_main\n",
    true,
    "deadbeef",
  );
  assert.ok(output.includes("_sfc_main.__scopeId = \"data-v-deadbeef\""));
}

console.log("utils.test.ts passed");
