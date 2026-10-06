(line_comment) @comment
(block_comment) @comment

(identifier) @variable
(string) @string
(escape_sequence) @string.escape
(number) @number
(boolean) @boolean
(null) @constant.builtin

(version_directive "layup" @keyword)
(declaration_keyword) @keyword
(connection_keyword) @keyword
(declaration keyword: (identifier) @type)
(annotation_argument name: (identifier) @property)
(attribute name: (identifier) @property)
(record_entry name: (identifier) @property)
(record_entry name: (string) @property)
(annotation_name (identifier) @attribute)
(attribute value: (reference (identifier) @constant))

(arrow) @operator
["=" ":"] @operator
["@" "." "::" "/" "," ";"] @punctuation.delimiter
["{" "}" "[" "]" "(" ")"] @punctuation.bracket

(annotation_argument value: (reference (identifier) @constant))
