use iota::iota;

iota! {
    pub const CONSTANT: u8 = iota;
            , NIL
            , TRUE
            , FALSE
            , POP
            , GET_LOCAL
            , SET_LOCAL
            , GET_GLOBAL
            , DEFINE_GLOBAL
            , SET_GLOBAL
            , GET_UPVALUE
            , SET_UPVALUE
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
            , JUMP
            , JUMP_IF_FALSE
            , LOOP
            , CALL
            , CLOSURE
            , RETURN
}
