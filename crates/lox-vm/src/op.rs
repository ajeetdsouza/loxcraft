#[derive(Clone, Copy, Debug)]
pub enum Op {
    /// Reads a constant at idx and pushes it onto the stack.
    Constant(ConstantIdx),
    /// Pushes a `nil` value onto the stack.
    Nil,
    /// Pushes a `false` value onto the stack.
    False,
    /// Pushes a `true` value onto the stack.
    True,
    /// Pops a value from the stack.
    Pop,

    /// Reads the stack at `idx` and pushes the value onto the stack.
    GetLocal(StackIdx),
    /// Pops a value from the stack and sets the stack at `idx` to it.
    SetLocal(StackIdx),
    /// Reads a constant (variable name) at `idx`, looks up its value in the
    /// `globals` map, and pushes the value onto the stack.
    GetGlobal(ConstantIdx),
    /// Reads a constant (variable name) at `idx`, pops a value from the stack,
    /// and sets the variable to this value in the `globals` map.
    DefineGlobal(ConstantIdx),
    /// Similar to [`Op::DefineGlobal`], but requires that the variable already
    /// exists in the `globals` map.
    SetGlobal(ConstantIdx),

    // Operators.
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,

    /// Pops a value from the stack and prints it to stdout.
    Print,
    /// Jumps forward by `offset` instructions.
    Jump(JumpOffset),
    /// Peeks a value from the stack. If it is `false`, jumps forward by
    /// `offset` instructions.
    JumpIfFalse(JumpOffset),
    /// Jumps backward by `offset` instructions.
    Loop(JumpOffset),
    /// Reads the number of arguments (`n`). It expects the stack to contain the
    /// function to be called followed by `n` arguments. It then pushes a new
    /// frame and calls the function in that frame.
    Call(ArgCount),
    /// Reads a constant (function) at idx, wraps it in a closure, and pushes it
    /// to the stack.
    Closure(ConstantIdx),
    /// Pops the current frame, and clears its stack. Retains the return value
    /// at the top of the stack.
    Return,
}

/// Used to represent the number of function arguments.
pub type ArgCount = u8;

/// Used as an index for the `constants` array.
pub type ConstantIdx = u8;

/// Used as an index for the `stack` array.
pub type StackIdx = u8;

/// Used as an offset for jump instructions.
pub type JumpOffset = u16;
