; Class declaration
(decl_class) @fold
(decl_class
  (function
  )? @fold
)

; Function declaration
(function
  params: (params) @fold
)
(function
  body: (stmt_block) @fold
)

; Block statement
(program
  (decl
    (decl_stmt
      (stmt_block) @fold
    )
  )
)
(stmt_block
  (decl
    (decl_stmt
      (stmt_block) @fold
    )
  )
)

; For statement
(stmt_for
  paren: (for_paren) @fold
)
(stmt_for
  body: (decl_stmt) @fold
)

; If statement
(stmt_if
  then: (decl_stmt) @fold
)
(stmt_if
  else: (decl_stmt) @fold
)

; While statement
(stmt_while
  body: (decl_stmt) @fold
)

; Function/Method call
(expr_call
  args: (args) @fold
)

; Grouping Expression
(grouping) @fold