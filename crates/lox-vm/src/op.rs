use iota::iota;

iota! {
    pub const
    // Reads a 1-byte constant index, and pushes the constant at that index onto
    // the stack.
    CONSTANT: u8 = iota;,
    // Pushes a nil value onto the stack.
    NIL,
    // Pushes a true value onto the stack.
    TRUE,
    // Pushes a false value to the stack.
    FALSE,
    // Pops a value from the stack.
    POP,
    // Reads a 1-byte stack slot, and pushes the value at that slot onto the
    // stack.
    GET_LOCAL,
    // Reads a 1-byte stack slot, and peeks at the value on top of the stack.
    // Sets the value at the stack slot to the value on top of the stack.
    SET_LOCAL,
    GET_GLOBAL,
    DEFINE_GLOBAL,
    SET_GLOBAL,
    GET_UPVALUE,
    SET_UPVALUE,
    GET_PROPERTY,
    SET_PROPERTY,
    GET_SUPER,
    // Pops 2 values from the stack, tests them for equality, and pushes the
    // result onto the stack.
    EQUAL,
    // Pops 2 values from the stack, tests them for inequality, and pushes the
    // result onto the stack.
    NOT_EQUAL,
    // Pops 2 values from the stack, tests the second for being greater than the
    //  first, and pushes the result onto the stack.
    GREATER,
    // Pops 2 values from the stack, tests the second for being greater than or
    // equal to the first, and pushes the result onto the stack.
    GREATER_EQUAL,
    // Pops 2 values from the stack, tests the second for being less than the
    // first, and pushes the result onto the stack.
    LESS,
    // Pops 2 values from the stack, tests the second for being less than or
    // equal to the first, and pushes the result onto the stack.
    LESS_EQUAL,
    // Pops 2 values from the stack, adds (in case of numbers) or concatenates
    // (in case of strings) them, and pushes the result onto the stack.
    ADD,
    // Pops 2 numbers from the stack, subtracts the first from the second, and
    // pushes the result onto the stack.
    SUBTRACT,
    // Pops 2 numbers from the stack, multiplies them, and pushes the result
    // onto the stack.
    MULTIPLY,
    // Pops 2 numbers from the stack, divides the second by the first, and
    // pushes the result onto the stack.
    DIVIDE,
    // Pops a value from the stack, checks if it is "falsey", and pushes the
    // result onto the stack.
    NOT,
    // Pops a number from the stack, negates it, and pushes the result onto the
    // stack.
    NEGATE,
    // Pops a value from the stack and prints it.
    PRINT,
    // Reads a 2-byte offset, and increments the instruction pointer by that
    // offset.
    JUMP,
    // Reads a 2-byte offset, and peeks at the value on top of the stack. If the
    // value is falsey, increments the instruction pointer by that offset.
    JUMP_IF_FALSE,
    // Reads a 2-byte offset, and decrements the instruction pointer by that
    // offset.
    LOOP,
    CALL,
    INVOKE,
    SUPER_INVOKE,
    CLOSURE,
    CLOSE_UPVALUE,
    RETURN,
    CLASS,
    INHERIT,
    METHOD
}
