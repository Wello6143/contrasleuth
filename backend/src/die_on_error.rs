use std::fmt::Debug;
use std::process::exit;
pub fn die_on_error<T, U: Debug>(result: Result<T, U>) -> T {
    match result {
        Ok(ok) => ok,
        Err(error) => {
            crate::log::fatal(format!("Unexpected error: {:?}", error));
            exit(1);
        }
    }
}
