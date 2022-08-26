; Comments
(comment) @comment

; Keywords
[
  "class"
  "else"
  "for"
  "fun"
  "if"
  "print"
  "return"
  "super"
  "var"
  "while"
  (this)
] @keyword

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "="
  "!"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "and"
  "or"
] @operator

; Punctuations
[
  "{"
  "}"
  "("
  ")"
  ","
  "."
  ";"
] @punctuation

; Literals
(string) @string
(number) @constant
(nil) @constant
(bool) @constant

; Function / method declaration
(function
  name: (identifier) @function
)

; Function call
(expr_call
  (expr_primary
    (var
      name: (identifier) @function
    )
  )
)

; Class declaration
(decl_class
  name: (identifier) @class
  extends: (extends)? @keyword
  base: (identifier)? @class
)

; Function declaration
(function
  name: (identifier) @function
)

; Method call
(expr_call
  callee: (expr_field
    field: (identifier) @function
  )
)
(expr_call
  callee: (expr_primary
    (super
      field: (identifier) @function
    )
  )
)

; Field access
(expr_field
  field: (identifier) @variable
)
(super
  field: (identifier) @variable
)

; Variable
(identifier) @variable
