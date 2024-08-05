//! Sub-package code (with feature)

#[cfg(feature: 'test_feature')]
fn test() {
    println!("test");
}

fn main() {
    println!("hello_world");
}
