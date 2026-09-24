export type Theme = 'light' | 'dark' | 'auto';

export interface RenderOptions {
  theme?: Theme;
  /** `svg`, a standalone interactive `html` page, or `embed` (HTML without toolbar, for an iframe). */
  format?: 'svg' | 'html' | 'embed';
}

export interface Diagnostic {
  line: number | null;
  message: string;
}

export interface RenderResult {
  output: string;
  warnings: Diagnostic[];
}

export interface Layup {
  /** Throws `LayupError` for invalid source. */
  render(source: string, options?: RenderOptions): RenderResult;
}

export class LayupError extends Error {
  line: number | null;
}

export function load(url?: string | URL): Promise<Layup>;
/** Node only. */
export function loadSync(): Layup;
