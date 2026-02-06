import fs from "node:fs";
import path from "node:path";

export async function findArtFiles(
  root: string,
  include: string[],
  exclude: string[],
): Promise<string[]> {
  const files: string[] = [];

  async function scan(dir: string): Promise<void> {
    const entries = await fs.promises.readdir(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const relative = path.relative(root, fullPath);

      let excluded = false;
      for (const pattern of exclude) {
        if (matchGlob(relative, pattern) || matchGlob(entry.name, pattern)) {
          excluded = true;
          break;
        }
      }

      if (excluded) continue;

      if (entry.isDirectory()) {
        await scan(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".art.vue")) {
        for (const pattern of include) {
          if (matchGlob(relative, pattern)) {
            files.push(fullPath);
            break;
          }
        }
      }
    }
  }

  await scan(root);
  return files;
}

function matchGlob(filepath: string, pattern: string): boolean {
  const regex = pattern
    .replace(/\*\*/g, "{{DOUBLE_STAR}}")
    .replace(/\*/g, "[^/]*")
    .replace(/{{DOUBLE_STAR}}/g, ".*")
    .replace(/\./g, "\\.");

  return new RegExp(`^${regex}$`).test(filepath);
}
