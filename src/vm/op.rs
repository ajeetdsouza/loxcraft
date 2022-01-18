use iota::iota;

iota! {
    pub const CONSTANT: u8 = iota;
            , NIL
            , FALSE
            , TRUE
            , POP
            , GET_LOCAL
            , SET_LOCAL
            , GET_GLOBAL
            , DEFINE_GLOBAL
            , SET_GLOBAL
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
            , JUMP
            , JUMP_IF_FALSE
            , LOOP
            , RETURN
}
