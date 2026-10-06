import type Parser from 'tree-sitter';

/** Native Layup language, compatible with the tree-sitter 0.25 Node runtime. */
declare const binding: Parser.Language & {
  /** Standard Tree-sitter highlight captures. */
  HIGHLIGHTS_QUERY: string;
  /** Folding ranges for blocks, values, comments and strings. */
  FOLDS_QUERY: string;
};

export default binding;
