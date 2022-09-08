use iota::iota;

iota! {
    pub const CONSTANT: u8 = iota;
            , NIL
            , TRUE
            , FALSE
            , POP
            , EQUAL
            , NOT_EQUAL
            , GREATER
            , GREATER_EQUAL
            , LESS
            , LESS_EQUAL
            , ADD
            , SUBTRACT
            , MULTIPLY
            , DIVIDE
            , NOT
            , NEGATE
            , PRINT
            , RETURN
}
