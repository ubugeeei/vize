import type { CompiledModule } from './types.js';

/**
 * HMR update types for granular hot module replacement.
 *
 * - 'template-only': Only template changed, use rerender (preserves state)
 * - 'style-only': Only styles changed, inject CSS without component remount
 * - 'full-reload': Script changed, full component reload required
 */
export type HmrUpdateType = 'template-only' | 'style-only' | 'full-reload';

/**
 * Detect the type of HMR update needed based on content hash changes.
 *
 * @param prev - Previously compiled module (undefined if first compile)
 * @param next - Newly compiled module
 * @returns The type of HMR update needed
 */
export function detectHmrUpdateType(
  prev: CompiledModule | undefined,
  next: CompiledModule
): HmrUpdateType {
  // First compile always requires full reload
  if (!prev) {
    return 'full-reload';
  }

  // Check for script changes (requires full reload)
  const scriptChanged = prev.scriptHash !== next.scriptHash;
  if (scriptChanged) {
    return 'full-reload';
  }

  // Check for template changes (can use rerender)
  const templateChanged = prev.templateHash !== next.templateHash;

  // Check for style changes
  const styleChanged = prev.styleHash !== next.styleHash;

  // If only style changed, we can do style-only update
  if (styleChanged && !templateChanged) {
    return 'style-only';
  }

  // If only template changed (or template + style), use rerender
  if (templateChanged) {
    return 'template-only';
  }

  // No changes detected (shouldn't happen in practice)
  return 'full-reload';
}

/**
 * Generate HMR-aware code output based on update type.
 */
export function generateHmrCode(
  scopeId: string,
  updateType: HmrUpdateType
): string {
  return `
if (import.meta.hot) {
  _sfc_main.__hmrId = ${JSON.stringify(scopeId)};
  _sfc_main.__hmrUpdateType = ${JSON.stringify(updateType)};

  import.meta.hot.accept((mod) => {
    if (!mod) return;
    const { default: updated } = mod;
    if (typeof __VUE_HMR_RUNTIME__ !== 'undefined') {
      const updateType = updated.__hmrUpdateType || 'full-reload';
      if (updateType === 'template-only') {
        __VUE_HMR_RUNTIME__.rerender(updated.__hmrId, updated.render);
      } else {
        __VUE_HMR_RUNTIME__.reload(updated.__hmrId, updated);
      }
    }
  });

  import.meta.hot.on('vize:update', (data) => {
    if (data.id !== _sfc_main.__hmrId) return;

    if (data.type === 'style-only') {
      // Update styles without remounting component
      const styleId = 'vize-style-' + _sfc_main.__hmrId;
      const styleEl = document.getElementById(styleId);
      if (styleEl && data.css) {
        styleEl.textContent = data.css;
      }
    }
  });

  if (typeof __VUE_HMR_RUNTIME__ !== 'undefined') {
    __VUE_HMR_RUNTIME__.createRecord(_sfc_main.__hmrId, _sfc_main);
  }
}`;
}
