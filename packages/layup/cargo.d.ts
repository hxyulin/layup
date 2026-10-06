import type { GraphInput, GraphViewInput, GraphEdgeInput, AnalysisProvenance } from './index.js';

export interface CargoAnalysis extends GraphInput {
  views: readonly GraphViewInput[];
  edges: readonly GraphEdgeInput[];
  provenance: AnalysisProvenance;
}

export interface CargoGraphOptions {
  /** Include direct external dependencies of workspace packages; default false. */
  includeExternal?: boolean;
  /** Include development dependencies; normal/build dependencies are always included. */
  includeDev?: boolean;
  /** Repository/source-root URL, ending in /; defaults to the local workspace file URL. */
  sourceBaseUrl?: string;
}
export interface AnalyzeCargoOptions extends CargoGraphOptions {
  manifestPath?: string;
  /** Defaults to true. Cargo metadata always uses --locked. */
  offline?: boolean;
  features?: readonly string[];
  noDefaultFeatures?: boolean;
  allFeatures?: boolean;
}
/** Node only. Runs cargo metadata without building or executing project code. */
export function analyzeCargo(options?: AnalyzeCargoOptions): Promise<CargoAnalysis>;
/** Adapts a Cargo metadata v1 object with resolve.nodes (not --no-deps output). */
export function cargoGraph(metadata: unknown, options?: CargoGraphOptions): CargoAnalysis;
