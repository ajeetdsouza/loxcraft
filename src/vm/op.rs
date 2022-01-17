use iota::iota;

iota! {
    pub const CONSTANT: u8 = iota;
            , NIL
            , FALSE
            , TRUE
            , POP
            , GET_GLOBAL
            , DEFINE_GLOBAL
            , EQUAL
            , GREATER
            , LESS
            , ADD
            , SUBTRACT
            , MULTIPLY
            , DIVIDE
            , NOT
            , NEGATE
            , PRINT
            , RETURN
}
