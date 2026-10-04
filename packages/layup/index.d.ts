export type Theme = 'light' | 'dark' | 'auto';

export interface RenderOptions {
  /** Optional fallback OpenType/TrueType font bytes, used for exact measurement and embedded in output. */
  fonts?: readonly Uint8Array[];
  theme?: Theme;
  /** `svg`, a standalone interactive `html` page, or `embed` (HTML without toolbar, for an iframe). */
  format?: 'svg' | 'html' | 'embed';
  /**
   * With `theme: 'auto'` and SVG output: switch to dark colors under an
   * ancestor matching this selector (such as `.dark`) instead of following
   * `prefers-color-scheme`.
   */
  darkSelector?: string;
}

/** UTF-8 byte offsets, with 1-based Unicode scalar line/column positions. */
export interface SourceSpan {
  start: number;
  end: number;
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}

export interface Diagnostic {
  line: number | null;
  message: string;
}

export interface LintDiagnostic extends Diagnostic {
  column: number | null;
  severity: 'error' | 'warning';
  code: string;
  span: SourceSpan | null;
  help: string | null;
  related: { span: SourceSpan; message: string }[];
}

export interface RenderResult {
  output: string;
  warnings: Diagnostic[];
}

export interface Layup {
  /** Throws `LayupError` for invalid source. */
  render(source: string, options?: RenderOptions): RenderResult;
  /** Format valid syntax without changing comments, strings or statement order. Throws LayupError. */
  format(source: string): string;
  /** Collect recoverable syntax errors, or semantic/layout and authoring diagnostics. Does not throw for invalid DSL. */
  lint(source: string, options?: Pick<RenderOptions, 'fonts'>): LintDiagnostic[];
}

export class LayupError extends Error {
  line: number | null;
  /** The message without its line prefix. */
  reason: string;
  column: number | null;
  span: SourceSpan | null;
  code: string | null;
  help: string | null;
  related: { span: SourceSpan; message: string }[];
}

export function load(url?: string | URL): Promise<Layup>;
/** Node only. */
export function loadSync(): Layup;
