import type { Theme } from './index.js';

export interface LayupMarkdownOptions {
  /** Default `auto`. */
  theme?: Theme;
  /** Follow a host class such as `.dark` instead of `prefers-color-scheme`. */
  darkSelector?: string;
  /** Emit markup that survives Vue template compilation. */
  vue?: boolean;
  /** Throw on warnings and errors to fail the build; otherwise they are logged. */
  strict?: boolean;
}

/**
 * markdown-it plugin rendering ```layup fences to inline SVG. A
 * ```layup source fence also shows its source as a code block.
 */
export default function layup(md: any, options?: LayupMarkdownOptions): void;

/** The plugin with VitePress defaults: `darkSelector: '.dark'`, `vue: true`. */
export function vitepress(md: any, options?: LayupMarkdownOptions): void;
