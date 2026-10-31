use rustali_macros::Entity;

#[derive(Entity)]
#[entity(crate = "rustali_orm")]
struct Point(i64, i64);

fn main() {}
