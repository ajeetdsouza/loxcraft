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
  callee: (expr_attribute
    attribute: (identifier) @function
  )
)
(expr_call
  callee: (expr_primary
    (super
      attribute: (identifier) @function
    )
  )
)

; Field access
(expr_attribute
  attribute: (identifier) @variable
)
(super
  attribute: (identifier) @variable
)

; Variable
(identifier) @variable
