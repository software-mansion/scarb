use std::cmp::Reverse;
use std::sync::Mutex;

/// Returns a small, process‑local numeric identifier for the current thread.
///
/// Properties:
/// - Unique among all threads that are alive at the same time.
/// - Stable for the lifetime of the current thread.
/// - Prefers to assign and reuse the lowest available numbers, starting from 0.
///
/// After a thread terminates, its identifier may be recycled and assigned to a future thread.
/// Don't persist this value for long-term storage or rely on it across process boundaries.
///
/// This is different from `std::thread::ThreadId` and doesn't guarantee stability across runs.
pub fn current() -> u64 {
    thread_local! {
        static CURRENT: ReusableThreadId = ReusableThreadId::borrow();
    }
    CURRENT.with(|id| id.0)
}

struct ReusableThreadId(u64);

struct State {
    /// This is always greater than any id ever used yet.
    next_free: u64,
    /// A list of ids that are free to borrow, with the last one being the first to pick.
    free_list: Vec<u64>,
}

static STATE: Mutex<State> = Mutex::new(State {
    next_free: 0,
    free_list: Vec::new(),
});

impl ReusableThreadId {
    fn borrow() -> Self {
        let mut state = STATE.lock().unwrap();
        // Try to reuse the lowest free id; otherwise, allocate a new one.
        match state.free_list.pop() {
            Some(id) => Self(id),
            None => {
                let id = state.next_free;
                state.next_free += 1;
                Self(id)
            }
        }
    }
}

impl Drop for ReusableThreadId {
    fn drop(&mut self) {
        let mut state = STATE.lock().unwrap();
        // Put the id back into the free list, ensuring that the lowest one gets popped first.
        // By striving to reuse lowest ids first, this code avoids the need to allocate new ones.
        state.free_list.push(self.0);
        state.free_list.sort_by_key(|id| Reverse(*id));
    }
}
