#[cfg(all(target_pointer_width = "64", feature = "jit"))]
pub fn put_value(val: i64) {
    print!("{}", val);
}

#[cfg(all(target_pointer_width = "32", feature = "jit"))]
pub fn put_value(val: i32) {
    print!("{}", val);
}

#[cfg(all(target_pointer_width = "64", feature = "jit"))]
pub fn put_char(c: i64) {
    let c = (c.abs() % std::u8::MAX as i64) as u8;
    print!("{}", c as char);
}

#[cfg(all(target_pointer_width = "32", feature = "jit"))]
pub fn put_char(c: i32) {
    let c = (c.abs() % std::u8::MAX as i32) as u8;
    print!("{}", c as char);
}
