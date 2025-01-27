use predicates::function::function;
use predicates::Predicate;
use std::fs;
use std::path::Path;

pub fn file_not_empty() -> impl Predicate<Path> {
    function(|path| fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false))
}
