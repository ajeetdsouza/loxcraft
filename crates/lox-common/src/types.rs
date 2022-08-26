use std::ops::Range;

pub type Spanned<T> = (T, Span);
pub type Span = Range<usize>;
