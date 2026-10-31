use rustali::prelude::*;

pub struct Ctl;

#[controller("/x")]
impl Ctl {
    #[get("/")]
    async fn index(&self) -> &'static str {
        "hi"
    }
}

fn main() {}
