import { createHash } from 'node:crypto';
import type { CompiledModule } from './types.js';
import { type HmrUpdateType, generateHmrCode } from './hmr.js';

export function generateScopeId(filename: string): string {
  const hash = createHash('sha256').update(filename).digest('hex');
  return hash.slice(0, 8);
}

export function createFilter(
  include?: string | RegExp | (string | RegExp)[],
  exclude?: string | RegExp | (string | RegExp)[]
): (id: string) => boolean {
  const includePatterns = include
    ? Array.isArray(include)
      ? include
      : [include]
    : [/\.vue$/];
  const excludePatterns = exclude
    ? Array.isArray(exclude)
      ? exclude
      : [exclude]
    : [/node_modules/];

  return (id: string) => {
    const matchInclude = includePatterns.some((pattern) =>
      typeof pattern === 'string' ? id.includes(pattern) : pattern.test(id)
    );
    const matchExclude = excludePatterns.some((pattern) =>
      typeof pattern === 'string' ? id.includes(pattern) : pattern.test(id)
    );
    return matchInclude && !matchExclude;
  };
}

export interface GenerateOutputOptions {
  isProduction: boolean;
  isDev: boolean;
  hmrUpdateType?: HmrUpdateType;
  extractCss?: boolean;
}

export function generateOutput(
  compiled: CompiledModule,
  options: GenerateOutputOptions
): string {
  const { isProduction, isDev, hmrUpdateType, extractCss } = options;

  let output = compiled.code;

  // Rewrite "export default" to named variable for HMR
  const hasExportDefault = output.includes('export default');
  if (hasExportDefault) {
    output = output.replace('export default', 'const _sfc_main =');
    // Add __scopeId for scoped CSS support
    if (compiled.hasScoped && compiled.scopeId) {
      output += `\n_sfc_main.__scopeId = "data-v-${compiled.scopeId}";`;
    }
    output += '\nexport default _sfc_main;';
  }

  // Inject CSS (skip in production if extracting)
  if (compiled.css && !(isProduction && extractCss)) {
    const cssCode = JSON.stringify(compiled.css);
    const cssId = JSON.stringify(`vize-style-${compiled.scopeId}`);
    output = `
const __vize_css__ = ${cssCode};
const __vize_css_id__ = ${cssId};
(function() {
  if (typeof document !== 'undefined') {
    let style = document.getElementById(__vize_css_id__);
    if (!style) {
      style = document.createElement('style');
      style.id = __vize_css_id__;
      style.textContent = __vize_css__;
      document.head.appendChild(style);
    } else {
      style.textContent = __vize_css__;
    }
  }
})();
${output}`;
  }

  // Add HMR support in development (skip in production)
  if (!isProduction && isDev && hasExportDefault) {
    output += generateHmrCode(compiled.scopeId, hmrUpdateType ?? 'full-reload');
  }

  return output;
}

/**
 * Legacy generateOutput signature for backward compatibility.
 */
export function generateOutputLegacy(
  compiled: CompiledModule,
  isProduction: boolean,
  isDev: boolean
): string {
  return generateOutput(compiled, { isProduction, isDev });
}
