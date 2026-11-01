//! Relation loading (`#[has_many]` / `#[belongs_to]` / `#[has_one]`) end to end
//! against an in-memory SQLite database — both lazy and eager.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rustali_macros::Entity;
use rustali_orm::sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema};

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("authors")]
#[has_many(Post, to = author_id)]
#[has_one(Profile, to = author_id)]
struct Author {
    id: i64,
    name: String,
}

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("profiles")]
#[belongs_to(Author, from = author_id)]
struct Profile {
    id: i64,
    author_id: i64,
    bio: String,
}

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("posts")]
#[belongs_to(Author, from = author_id, as = author)]
#[has_many(Comment, to = post_id)]
#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]
struct Post {
    id: i64,
    author_id: i64,
    title: String,
}

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("tags")]
#[many_to_many(Post, via = PostTag, from = tag_id, to = post_id)]
struct Tag {
    id: i64,
    label: String,
}

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("post_tags")]
#[belongs_to(Post, from = post_id)]
#[belongs_to(Tag, from = tag_id)]
struct PostTag {
    id: i64,
    post_id: i64,
    tag_id: i64,
}

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("comments")]
#[belongs_to(Post, from = post_id)]
struct Comment {
    id: i64,
    post_id: i64,
    body: String,
}

async fn db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    let schema = Schema::new(DbBackend::Sqlite);
    for stmt in [
        schema.create_table_from_entity(author_entity::Entity),
        schema.create_table_from_entity(profile_entity::Entity),
        schema.create_table_from_entity(post_entity::Entity),
        schema.create_table_from_entity(comment_entity::Entity),
        schema.create_table_from_entity(tag_entity::Entity),
        schema.create_table_from_entity(post_tag_entity::Entity),
    ] {
        db.execute(&stmt).await.expect("create table");
    }
    db
}

async fn seed(db: &DatabaseConnection) -> Author {
    let author = Author {
        id: 0,
        name: "Ada".into(),
    }
    .insert(db)
    .await
    .unwrap();
    Profile {
        id: 0,
        author_id: author.id,
        bio: "pioneer".into(),
    }
    .insert(db)
    .await
    .unwrap();
    for i in 0..3 {
        let post = Post {
            id: 0,
            author_id: author.id,
            title: format!("post {i}"),
        }
        .insert(db)
        .await
        .unwrap();
        for j in 0..2 {
            Comment {
                id: 0,
                post_id: post.id,
                body: format!("c{i}-{j}"),
            }
            .insert(db)
            .await
            .unwrap();
        }
    }
    author
}

#[tokio::test]
async fn lazy_has_many() {
    let db = db().await;
    let author = seed(&db).await;
    let posts = author.posts(&db).await.unwrap();
    assert_eq!(posts.len(), 3);
    let comments = posts[0].comments(&db).await.unwrap();
    assert_eq!(comments.len(), 2);
}

#[tokio::test]
async fn lazy_belongs_to_and_has_one() {
    let db = db().await;
    let author = seed(&db).await;
    let posts = author.posts(&db).await.unwrap();
    // belongs_to with `as = author`
    let owner = posts[0].author(&db).await.unwrap();
    assert_eq!(owner.as_ref().map(|a| a.name.as_str()), Some("Ada"));
    // has_one
    let profile = author.profile(&db).await.unwrap();
    assert_eq!(profile.as_ref().map(|p| p.bio.as_str()), Some("pioneer"));
}

#[tokio::test]
async fn eager_has_many_avoids_n_plus_one() {
    let db = db().await;
    seed(&db).await;
    let posts = Post::all(&db).await.unwrap();
    let paired = Post::with_comments(posts, &db).await.unwrap();
    assert_eq!(paired.len(), 3);
    for (_post, comments) in &paired {
        assert_eq!(comments.len(), 2);
    }
}

#[tokio::test]
async fn eager_belongs_to() {
    let db = db().await;
    seed(&db).await;
    let posts = Post::all(&db).await.unwrap();
    let paired = Post::with_author(posts, &db).await.unwrap();
    assert_eq!(paired.len(), 3);
    for (_post, author) in &paired {
        assert_eq!(author.as_ref().map(|a| a.name.as_str()), Some("Ada"));
    }
}

async fn seed_tags(db: &DatabaseConnection) {
    let author = Author {
        id: 0,
        name: "Ada".into(),
    }
    .insert(db)
    .await
    .unwrap();
    let rust = Tag {
        id: 0,
        label: "rust".into(),
    }
    .insert(db)
    .await
    .unwrap();
    let web = Tag {
        id: 0,
        label: "web".into(),
    }
    .insert(db)
    .await
    .unwrap();
    let p1 = Post {
        id: 0,
        author_id: author.id,
        title: "a".into(),
    }
    .insert(db)
    .await
    .unwrap();
    let p2 = Post {
        id: 0,
        author_id: author.id,
        title: "b".into(),
    }
    .insert(db)
    .await
    .unwrap();
    // p1 -> {rust, web}, p2 -> {rust}
    for (post_id, tag_id) in [(p1.id, rust.id), (p1.id, web.id), (p2.id, rust.id)] {
        PostTag {
            id: 0,
            post_id,
            tag_id,
        }
        .insert(db)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn lazy_many_to_many() {
    let db = db().await;
    seed_tags(&db).await;
    let posts = Post::all(&db).await.unwrap();
    let p1_tags = posts[0].tags(&db).await.unwrap();
    assert_eq!(p1_tags.len(), 2);
    let p2_tags = posts[1].tags(&db).await.unwrap();
    assert_eq!(p2_tags.len(), 1);
    // the reverse direction works too
    let tags = Tag::all(&db).await.unwrap();
    let rust_posts = tags[0].posts(&db).await.unwrap();
    assert_eq!(rust_posts.len(), 2);
}

#[tokio::test]
async fn eager_many_to_many_avoids_n_plus_one() {
    let db = db().await;
    seed_tags(&db).await;
    let posts = Post::all(&db).await.unwrap();
    let paired = Post::with_tags(posts, &db).await.unwrap();
    let counts: Vec<usize> = paired.iter().map(|(_, tags)| tags.len()).collect();
    assert_eq!(counts, [2, 1]);
}
