//! Server-rendered posts with validation, flash and CSRF.

use askama::Template;
use rustali::{prelude::*, session::flash::Message};
use serde::Deserialize;

use crate::models::Post;

#[derive(Template)]
#[template(path = "posts/index.html")]
struct PostsIndex {
    app_name: String,
    flash: Vec<Message>,
    posts: Vec<Post>,
    csrf_field: String,
    old_title: String,
    old_body: String,
    title_error: Option<String>,
    body_error: Option<String>,
}

#[derive(Deserialize, Validate)]
struct CreatePost {
    #[rule(required, length(min = 3, max = 200))]
    title: String,
    #[rule(required, length(min = 1))]
    body: String,
    // CSRF middleware reads this; accept it so the form deserialises.
    #[serde(default, rename = "_token")]
    _token: String,
}

pub struct PostController;

#[controller("/")]
impl PostController {
    #[get("/", name = "posts.index")]
    async fn index(ctx: Ctx, db: Db, flash: Flash, csrf: Csrf) -> Result<View<PostsIndex>> {
        let errors = flash.errors().await;
        let page = PostsIndex {
            app_name: ctx.config().app.name.clone(),
            flash: flash.take().await,
            posts: Post::query().order_by_desc(Post::ID).fetch_all(&db).await?,
            csrf_field: csrf.field().await,
            old_title: flash.old("title").await.unwrap_or_default(),
            old_body: flash.old("body").await.unwrap_or_default(),
            title_error: errors.first("title").map(str::to_owned),
            body_error: errors.first("body").map(str::to_owned),
        };
        flash.clear_old().await;
        Ok(View::new(page))
    }

    #[post("/posts", name = "posts.store")]
    async fn store(
        db: Db,
        flash: Flash,
        Validated(Form(input)): Validated<Form<CreatePost>>,
    ) -> Result<Redirect> {
        Post::new(input.title.trim(), input.body.trim())
            .insert(&db)
            .await?;
        flash.success("Post published.").await;
        Ok(Redirect::to("/"))
    }
}
