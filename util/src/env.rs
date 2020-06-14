use std::env;

pub fn var(key: &str) -> String {
    match env::var(key) {
        Ok(value) => value,
        Err(e) => panic!("couldn't interpret {}: {}", key, e),
    }
}
