use std::fmt;

pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

pub struct MyError {
    msg: String,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

pub trait Greeter {
    fn greet(&self) -> String;
}

pub enum Status {
    Ok,
    Err,
}
