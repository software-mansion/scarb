use serde::Serialize;

#[derive(Serialize)]
pub struct ErrResponse {
    message: String,
}

impl ErrResponse {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
