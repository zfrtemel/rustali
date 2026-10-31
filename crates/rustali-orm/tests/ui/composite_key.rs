use rustali_macros::Entity;

#[derive(Entity)]
#[entity(crate = "rustali_orm")]
struct Membership {
    #[key]
    user_id: i64,
    #[key]
    team_id: i64,
}

fn main() {}
