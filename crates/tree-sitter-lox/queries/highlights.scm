; Keywords
[
  "class"
  "print"
  "var"
  "super"
  (this)
] @keyword

"fun" @keyword.function
"return" @keyword.return

; Looping Keywords
[
  "for"
  "while"
] @repeat

; Conditional keywords
[
  "if"
  "else"
] @conditional

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
] @punctuation.bracket
[
  ","
  "."
  ";"
] @punctuation.delimiter

; Literals
(string) @string
(number) @number
(nil) @constant
(bool) @constant

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
  name: (identifier) @type
  base: (identifier)? @type
  (function
    name: (identifier) @method
  )?
)

; Function declaration
(function
  name: (identifier) @function
  params: (params (identifier)  @parameter)?
)

; Method call
(expr_call
  (expr_attribute
     attribute: (identifier) @method
  )
)

; Field access
(expr_attribute
  attribute: (identifier) @field
)
(super
  attribute: (identifier) @field
)
