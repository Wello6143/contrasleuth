use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::{DateTime, Utc};
use std::fmt::Display;
fn now() -> DelayedFormat<StrftimeItems<'static>> {
    let now: DateTime<Utc> = Utc::now();
    now.format("%b %e %T %Y")
}

pub fn welcome<T: Display>(message: T) {
    println!("{} [WELCOME] {}", now(), message);
}

pub fn notice<T: Display>(message: T) {
    println!("{} [NOTICE] {}", now(), message);
}

pub fn warning<T: Display>(message: T) {
    println!("{} [WARNING] {}", now(), message);
}

pub fn fatal<T: Display>(message: T) {
    println!("{} [FATAL] {}", now(), message);
}
