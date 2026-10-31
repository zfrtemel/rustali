use rustali_macros::Entity;

#[derive(Entity)]
#[entity(crate = "rustali_orm")]
struct Tag {
    name: String,
}

fn main() {}
