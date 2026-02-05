import { strict as assert } from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { vize } from "./index.js";

function makeTempProject() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "vize-plugin-"));
  const src = path.join(root, "src");
  fs.mkdirSync(src);
  const compPath = path.join(src, "Comp.vue");
  const utilPath = path.join(src, "util.ts");
  fs.writeFileSync(compPath, "<template><div/></template>");
  fs.writeFileSync(utilPath, "export const x = 1;\n");
  return { root, compPath, utilPath };
}

async function setupPlugin(root: string) {
  const plugin = vize();
  await plugin.configResolved?.({ root, isProduction: false } as any);
  return plugin;
}

(async () => {
  const { root, compPath, utilPath } = makeTempProject();
  const plugin = await setupPlugin(root);
  const importer = `\0vize:${compPath}.ts`;

  const resolved = await plugin.resolveId?.call(
    { resolve: async () => null } as any,
    "./util.ts?raw",
    importer,
  );
  assert.equal(resolved, utilPath + "?raw");

  let resolveCalled = false;
  const absId = utilPath + "?raw";
  const resolvedAbs = await plugin.resolveId?.call(
    {
      resolve: async () => {
        resolveCalled = true;
        return null;
      },
    } as any,
    absId,
    importer,
  );
  assert.equal(resolvedAbs, absId);
  assert.equal(resolveCalled, false);

  fs.rmSync(root, { recursive: true, force: true });

  console.log("index.test.ts passed");
})();
