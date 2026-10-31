use rustali::prelude::*;

pub struct Ctl;

#[controller("/x")]
impl Ctl {
    #[get("/")]
    fn index() -> &'static str {
        "hi"
    }
}

fn main() {}
