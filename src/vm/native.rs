use crate::vm::{Value, VM};

use iota::iota;

iota! {
    pub const CLOCK: u8 = iota;
}

pub type Function<W> = Option<fn(&VM<W>, &[Value]) -> Value>;

pub fn clock<W>(vm: &VM<W>, _args: &[Value]) -> Value {
    let elapsed = vm.start_time.elapsed();
    Value::Number(elapsed.as_secs_f64())
}
