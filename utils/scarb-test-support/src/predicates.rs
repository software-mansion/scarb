use predicates::function::function;
use predicates::Predicate;
use std::fs;
use std::path::Path;

pub fn non_empty_file() -> impl Predicate<Path> {
    function(|path| fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false))
}
