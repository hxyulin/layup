// Shared toolbar icons use SVG geometry rather than platform font glyphs.
const icon = path => `<svg class="layup-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${path}"/></svg>`;

export const EXPAND_ICON = icon('M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5');
export const ZOOM_IN_ICON = icon('M5 12h14M12 5v14');
export const ZOOM_OUT_ICON = icon('M5 12h14');
