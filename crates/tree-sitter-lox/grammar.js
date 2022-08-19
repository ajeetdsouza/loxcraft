// This grammar has been adapted from
// https://craftinginterpreters.com/appendix-i.html#syntax-grammar.

const precTable = {
  call: 8,
  prefix: 7,
  factor: 6,
  term: 5,
  comparison: 4,
  equality: 3,
  logic_and: 2,
  logic_or: 1,
  assign: 0,
};
const commaSep = (rule) => seq(rule, repeat(seq(",", rule)));

module.exports = grammar({
  name: "lox",
  extras: ($) => [
    /\s|\r?\n/,
    $.comment,
  ],
  rules: {
    // Program
    program: ($) => field("decl", repeat($.decl)),

    // Declarations
    decl: ($) =>
      choice(
        $.decl_class,
        $.decl_fun,
        $.decl_var,
        $.decl_stmt,
      ),
    decl_class: ($) =>
      seq(
        "class",
        field("name", $.identifier),
        optional(seq(field("extends", $.extends), field("base", $.identifier))),
        "{",
        field("method", repeat($.function)),
        "}"
      ),
    decl_fun: ($) => seq("fun", field("function", $.function)),
    decl_var: ($) =>
      seq(
        "var",
        field("name", $.identifier),
        optional(seq("=", field("value", $._expr))),
        ";"
      ),

    // Statements
    decl_stmt: ($) =>
      choice(
        $.stmt_block,
        $.stmt_expr,
        $.stmt_for,
        $.stmt_if,
        $.stmt_print,
        $.stmt_return,
        $.stmt_while
      ),
    stmt_block: ($) => seq("{", field("body", repeat($.decl)), "}"),
    stmt_expr: ($) => seq(field("value", $._expr), ";"),
    stmt_for: ($) =>
      seq(
        "for",
        field("paren", $.for_paren),
        field("body", $.decl_stmt)
      ),
    for_paren: ($) =>
      seq(
        "(",
        choice(field("init", choice($.stmt_expr, $.decl_var)), ";"),
        optional(field("cond", $._expr)),
        ";",
        optional(field("incr", choice($._expr))),
        ")",
      ),
    stmt_if: ($) =>
      prec.right(
        seq(
          "if",
          field("cond", $.grouping),
          field("then", $.decl_stmt),
          optional(seq("else", field("else", $.decl_stmt)))
        )
      ),
    stmt_print: ($) => seq("print", field("value", $._expr), ";"),
    stmt_return: ($) => seq("return", optional(field("value", $._expr)), ";"),
    stmt_while: ($) =>
      seq("while", field("cond", $.grouping), field("body", $.decl_stmt)),

    // Expressions
    _expr: ($) =>
      choice(
        $.expr_call,
        $.expr_infix,
        $.expr_prefix,
        $.expr_primary,
        $.expr_field
      ),
    expr_call: ($) =>
      prec.left(
        precTable.call,
        seq(field("callee", $._expr), field("args", $.args))
      ),
    expr_field: ($) => prec.left(precTable.call, seq(
      field("object", $._expr),
      '.',
      field("field", $.identifier)
    )),
    expr_infix: ($) => {
      const table = [
        [prec.left, precTable.factor, choice("*", "/")],
        [prec.left, precTable.term, choice("+", "-")],
        [prec.left, precTable.comparison, choice("<", "<=", ">", ">=")],
        [prec.left, precTable.equality, choice("==", "!=")],
        [prec.left, precTable.logic_and, "and"],
        [prec.left, precTable.logic_or, "or"],
        [prec.right, precTable.assign, "="],
      ];
      return choice(
        ...table.map(([precFn, precOp, op]) =>
          precFn(
            precOp,
            seq(field("lt", $._expr), field("op", op), field("rt", $._expr))
          )
        )
      );
    },
    expr_prefix: ($) =>
      prec.right(
        precTable.prefix,
        seq(field("op", choice("-", "!")), field("rt", $._expr))
      ),

    expr_primary: ($) => choice($.bool, $.nil, $.this, $.number, $.string, $.var, $.grouping, $.super),

    // Primary Expressions
    bool: ($) => choice("false", "true"),
    nil: ($) => "nil",
    this: ($) => "this",
    number: ($) => /[0-9]+(\.[0-9]+)?/,
    string: ($) => /"[^"]*"/,
    var: ($) => field("name", $.identifier),
    grouping: ($) => seq("(", field("inner", $._expr), ")"),
    super: ($) => seq("super", ".", field("field", $.identifier)),

    // Utilities
    extends: ($) => "<",
    function: ($) =>
      seq(
        field("name", $.identifier),
        field("params", $.params),
        field("body", $.stmt_block)
      ),
    args: ($) => seq("(", optional(commaSep($._expr)), ")"),
    params: ($) => seq("(", optional(commaSep($.identifier)), ")"),
    comment: ($) => token(seq("//", /.*/)),
    // Currently, this regex allows keywords to show up as identifiers in
    // certain contexts; i.e. statements like `var nil = "foo";` are allowed.
    // This can be fixed once tree-sitter adds support for reserved words:
    // https://github.com/tree-sitter/tree-sitter/pull/1635
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/
  },
  word: ($) => $.identifier,
});
