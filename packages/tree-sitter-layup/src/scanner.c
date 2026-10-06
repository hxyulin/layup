#include "tree_sitter/parser.h"
#include <stddef.h>

void *tree_sitter_layup_external_scanner_create(void) { return NULL; }
void tree_sitter_layup_external_scanner_destroy(void *payload) { (void)payload; }
unsigned tree_sitter_layup_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload; (void)buffer; return 0;
}
void tree_sitter_layup_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  (void)payload; (void)buffer; (void)length;
}

bool tree_sitter_layup_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  (void)payload;
  // An unused external token distinguishes Tree-sitter's all-valid recovery mode.
  if (valid_symbols[2]) return false;
  if (valid_symbols[1]) {
    if (lexer->eof(lexer) || lexer->lookahead == '"' || lexer->lookahead == '\\') return false;
    do {
      lexer->advance(lexer, false);
    } while (!lexer->eof(lexer) && lexer->lookahead != '"' && lexer->lookahead != '\\');
    lexer->mark_end(lexer);
    lexer->result_symbol = 1;
    return true;
  }
  if (!valid_symbols[0]) return false;
  // External tokens must skip the same horizontal whitespace as the grammar.
  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
         lexer->lookahead == '\r' || lexer->lookahead == '\f') {
    lexer->advance(lexer, true);
  }
  if (lexer->lookahead != '/') return false;
  lexer->advance(lexer, false);
  if (lexer->lookahead != '*') return false;
  lexer->advance(lexer, false);
  unsigned depth = 1;
  while (!lexer->eof(lexer)) {
    int32_t current = lexer->lookahead;
    lexer->advance(lexer, false);
    if (current == '/' && lexer->lookahead == '*') {
      depth++;
      lexer->advance(lexer, false);
    } else if (current == '*' && lexer->lookahead == '/') {
      lexer->advance(lexer, false);
      if (--depth == 0) {
        lexer->mark_end(lexer);
        lexer->result_symbol = 0;
        return true;
      }
    }
  }
  return false;
}
