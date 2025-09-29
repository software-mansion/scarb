use std::sync::atomic::{AtomicU64, Ordering};

wit_bindgen::generate!({
    inline: r#"
        package testing:oracle;

        world oracle {
            export add: func(left: u64, right: u64) -> u64;
            export join: func(a: string, b: string) -> string;
            export io: func();
            export count: func() -> u64;
        }
    "#
});

struct MyOracle;

impl Guest for MyOracle {
    fn add(left: u64, right: u64) -> u64 {
        left + right
    }
    fn join(a: String, b: String) -> String {
        a + &b
    }
    fn io() {
        println!("stdout is working as expected");
        eprintln!("stderr is working as expected");
    }
    fn count() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

export!(MyOracle);
