export type Theme = 'light' | 'dark' | 'auto';

export interface CompileOptions {
  /** Optional fallback OpenType/TrueType bytes used for exact measurement. */
  fonts?: readonly Uint8Array[];
  /** Select a named view of a shared model; the first authored view is the default. */
  view?: string;
  /** Select a diagram in a `layup 1` document; defaults to the first declaration. */
  diagram?: string;
}

export interface RenderOptions extends CompileOptions {
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
  lint(source: string, options?: CompileOptions): LintDiagnostic[];
  /** Return a versioned scene with geometry, drawing items, source spans, and presentation metadata. Throws LayupError. */
  compile(source: string, options?: CompileOptions): Scene;
  /** Validate and compile versioned semantic graph input through the same engine as the DSL. */
  compileModel(model: GraphInput, options?: CompileOptions): Scene;
  /** Render structured graph input; preserves selected analysis evidence in SVG metadata. */
  renderModel(model: GraphInput, options?: RenderOptions): RenderResult;
}

export type JsonValue = null | boolean | number | string | readonly JsonValue[] | { readonly [key: string]: JsonValue };
export interface SourceLocation {
  /** Absolute URI, or URI relative to the analyzer's source root. */
  uri: string;
  /** Omit when only the file is known. Lines/Unicode scalar columns are 1-based; end is exclusive. */
  range?: { startLine: number; startColumn: number; endLine: number; endColumn: number } | null;
  symbol?: string | null;
}
export interface AnalysisProvenance {
  analyzer: string;
  version?: string | null;
  metadata?: { readonly [key: string]: JsonValue };
}
export interface GraphNodeInput {
  /** Stable symbol ID; arbitrary nonempty strings are preserved without encoding. */
  id: string;
  title: string;
  /** Built-in graph kind, or a custom kind rendered as a card. Default node. */
  kind?: string;
  parentId?: string | null;
  code?: readonly string[];
  description?: readonly string[];
  role?: string | null;
  href?: string | null;
  tone?: Tone | null;
  sourceLocations?: readonly SourceLocation[];
  metadata?: { readonly [key: string]: JsonValue };
}
export interface GraphEdgeInput {
  id: string;
  from: string;
  to: string;
  /** Built-in relation, or a custom relation rendered as a blue arrow. Default flow. */
  kind?: string;
  label?: string | null;
  tone?: Tone | null;
  dashed?: boolean | null;
  sourceLocations?: readonly SourceLocation[];
  metadata?: { readonly [key: string]: JsonValue };
}
export interface GraphViewInput {
  id: string;
  title: string;
  include: readonly string[];
  direction?: 'down' | 'up' | 'right' | 'left' | null;
}
export interface GraphInput {
  version: 1;
  title: string;
  description?: string | null;
  /** Automatic graph flow; defaults to down. */
  direction?: 'down' | 'up' | 'right' | 'left';
  nodes: readonly GraphNodeInput[];
  edges?: readonly GraphEdgeInput[];
  views?: readonly GraphViewInput[];
  provenance?: AnalysisProvenance | null;
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


export type Tone = 'gray' | 'blue' | 'green' | 'yellow' | 'purple' | 'orange' | 'red';
export interface Point { readonly x: number; readonly y: number; }
export interface Rect extends Point { readonly width: number; readonly height: number; }
export type Outline =
  | { readonly type: 'rectangle'; readonly rect: Rect }
  | { readonly type: 'rounded'; readonly rect: Rect; readonly radius: number }
  | { readonly type: 'diamond'; readonly rect: Rect; readonly vertices: readonly Point[] };

/** Drawing coordinates belong to the original scene, before the optional slide transform. */
export type Drawing =
  | { readonly type: 'group'; readonly items: readonly Drawing[] }
  | { readonly type: 'box'; readonly rect: Rect; readonly tone: Tone; readonly hollow: boolean; readonly white: boolean; readonly radius: number; readonly strokeWidth: number }
  | { readonly type: 'state-marker'; readonly rect: Rect; readonly tone: Tone; readonly finalState: boolean }
  | { readonly type: 'diamond'; readonly rect: Rect; readonly tone: Tone; readonly hollow: boolean; readonly strokeWidth: number }
  | { readonly type: 'strip'; readonly rect: Rect; readonly radius: number }
  | { readonly type: 'rule'; readonly from: Point; readonly to: Point; readonly tone: Tone | null; readonly dashed: boolean }
  | { readonly type: 'text'; readonly x: number; readonly y: number; readonly anchor: 'start' | 'middle' | 'end'; readonly direction: 'auto' | 'ltr' | 'rtl'; readonly runs: readonly TextRun[]; readonly size: number; readonly weight: number; readonly mono: boolean; readonly ink: 'text' | 'muted' | 'code' | { readonly tone: Tone }; readonly letterSpacing: number }
  | { readonly type: 'chip'; readonly rect: Rect; readonly text: string; readonly tone: Tone; readonly rotate: boolean; readonly bordered: boolean }
  | { readonly type: 'sample'; readonly x: number; readonly y: number; readonly tone: Tone; readonly dashed: boolean };

export interface TextRun {
  readonly text: string;
  readonly code: boolean;
  readonly tag: boolean;
}
export interface SceneNode {
  readonly id: string;
  /** Authored path segments for revision-one DSL; renderer IDs remain opaque. */
  readonly objectPath: readonly string[] | null;
  readonly kind: string;
  readonly parentId: string | null;
  readonly rect: Rect;
  readonly outline: Outline;
  readonly tone: Tone;
  readonly href: string | null;
  readonly line: number | null;
  readonly span: SourceSpan | null;
  readonly sourceLocations: readonly SourceLocation[];
  readonly metadata: { readonly [key: string]: JsonValue };
}
export interface SceneEdge {
  readonly id: string;
  /** Named revision-one relationship ID; null for anonymous or legacy edges. */
  readonly authoredId: string | null;
  readonly from: string;
  readonly to: string;
  readonly kind: string;
  readonly points: readonly Point[];
  readonly style: { readonly tone: Tone; readonly dashed: boolean; readonly headStart: boolean; readonly headEnd: boolean; readonly bus: boolean; readonly asynchronous: boolean };
  readonly chip: Drawing | null;
  readonly line: number | null;
  readonly span: SourceSpan | null;
  readonly sourceLocations: readonly SourceLocation[];
  readonly metadata: { readonly [key: string]: JsonValue };
}
export interface PresentationStep {
  readonly id: string;
  readonly title: string;
  readonly note: string | null;
  readonly visibleNodes: readonly string[];
  readonly visibleEdges: readonly string[];
  readonly highlightNodes: readonly string[];
  readonly highlightEdges: readonly string[];
}
export interface Presentation {
  readonly version: 1;
  readonly steps: readonly PresentationStep[];
}
export interface SlideTransform {
  readonly scale: number;
  readonly offsetX: number;
  readonly offsetY: number;
  readonly padding: number;
  readonly contentWidth: number;
  readonly contentHeight: number;
  readonly minFontSize: number;
  readonly smallestFontSize: number | null;
}
export interface SceneView {
  readonly id: string;
  readonly title: string;
  readonly span: SourceSpan | null;
}
export interface SequenceInfo {
  readonly participants: readonly string[];
  readonly messages: readonly { readonly id: string; readonly asynchronous: boolean; readonly line: number; readonly span: SourceSpan }[];
  readonly annotations: readonly { readonly kind: string; readonly label: string; readonly rect: Rect; readonly line: number; readonly span: SourceSpan }[];
}
export interface DocumentEntityInfo {
  readonly path: readonly string[] | null;
  readonly authoredId: string | null;
  readonly documentation: string | null;
  readonly sourceLocations: readonly SourceLocation[];
  readonly metadata: { readonly [namespace: string]: JsonValue };
}
export interface DocumentInfo {
  readonly languageVersion: 1;
  readonly diagramId: string;
  readonly diagrams: readonly string[];
  readonly diagram: DocumentEntityInfo;
  readonly objects: { readonly [renderId: string]: DocumentEntityInfo };
  readonly relationships: { readonly [renderId: string]: DocumentEntityInfo };
}
export interface Scene {
  readonly version: 1;
  readonly units: 'svg-user-units';
  readonly coordinateSystem: 'scene';
  readonly title: string;
  readonly mode: 'graph' | 'state-machine' | 'sequence';
  readonly width: number;
  readonly height: number;
  readonly margin: number;
  readonly contentLeft: number;
  readonly contentRight: number;
  readonly viewport: { readonly width: number; readonly height: number; readonly slide: SlideTransform | null };
  readonly selectedView: string | null;
  readonly document: DocumentInfo | null;
  readonly provenance: AnalysisProvenance | null;
  readonly views: readonly SceneView[];
  readonly nodes: readonly SceneNode[];
  readonly edges: readonly SceneEdge[];
  readonly items: readonly { readonly nodeId: string | null; readonly drawing: Drawing }[];
  readonly keepout: readonly Rect[];
  readonly fonts: { readonly sans: string; readonly mono: string; readonly bundledFallbacks: readonly string[]; readonly fallbacks: readonly string[]; readonly systemCjk: boolean };
  readonly presentation: Presentation;
  readonly sequence: SequenceInfo | null;
  readonly diagnostics: readonly LintDiagnostic[];
}
