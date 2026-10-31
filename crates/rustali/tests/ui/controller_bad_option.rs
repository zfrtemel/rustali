use rustali::prelude::*;

pub struct Ctl;

#[controller("/x")]
impl Ctl {
    #[get("/", nmae = "typo")]
    async fn index() -> &'static str {
        "hi"
    }
}

fn main() {}
