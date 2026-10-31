use rustali_macros::Entity;

#[derive(Entity)]
#[entity(crate = "rustali_orm")]
struct Tag {
    id: i64,
    #[column(lenght = 10)]
    name: String,
}

fn main() {}
