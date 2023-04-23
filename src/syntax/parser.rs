use lalrpop_util::lalrpop_mod;

pub type Parser = grammar::ProgramParser;

lalrpop_mod!(
    #[allow(clippy::all)]
    grammar,
    "/res/grammar.rs"
);
