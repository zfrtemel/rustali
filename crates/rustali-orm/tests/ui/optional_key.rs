use rustali_macros::Entity;

#[derive(Entity)]
#[entity(crate = "rustali_orm")]
struct Tag {
    #[key]
    id: Option<i64>,
    name: String,
}

fn main() {}
