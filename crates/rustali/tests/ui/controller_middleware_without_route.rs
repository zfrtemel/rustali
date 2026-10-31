use rustali::prelude::*;

pub struct Ctl;

#[controller("/x")]
impl Ctl {
    #[get("/")]
    async fn index() -> &'static str {
        "hi"
    }

    #[middleware(Throttle::per_minute(1))]
    async fn helper() {}
}

fn main() {}
