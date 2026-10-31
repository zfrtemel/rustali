use rustali::prelude::*;

pub struct Ctl;

#[controller("/x")]
impl Ctl {
    async fn not_a_route() {}
}

fn main() {}
