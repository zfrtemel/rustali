//! Register, login, logout and a protected dashboard.

use askama::Template;
use rustali::{http::StatusCode, prelude::*};
use serde::Deserialize;

use crate::models::User;

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterPage {
    app_name: String,
    csrf_field: String,
    old_name: String,
    old_email: String,
    name_error: Option<String>,
    email_error: Option<String>,
    password_error: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    app_name: String,
    csrf_field: String,
    flash: Vec<rustali::session::Message>,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardPage {
    app_name: String,
    user_name: String,
    user_email: String,
    csrf_field: String,
}

#[derive(Deserialize, Validate)]
struct RegisterForm {
    #[rule(required, length(min = 2, max = 50))]
    name: String,
    #[rule(required, email)]
    email: String,
    #[rule(required, length(min = 8), confirmed)]
    password: String,
    #[serde(default)]
    password_confirmation: String,
    #[serde(default, rename = "_token")]
    _token: String,
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
    #[serde(default, rename = "_token")]
    _token: String,
}

pub struct AuthController;

#[controller("/")]
impl AuthController {
    #[get("/register", name = "register")]
    async fn register_form(ctx: Ctx, csrf: Csrf, flash: Flash) -> Result<View<RegisterPage>> {
        let errors = flash.errors().await;
        let page = RegisterPage {
            app_name: ctx.config().app.name.clone(),
            csrf_field: csrf.field().await,
            old_name: flash.old("name").await.unwrap_or_default(),
            old_email: flash.old("email").await.unwrap_or_default(),
            name_error: errors.first("name").map(str::to_owned),
            email_error: errors.first("email").map(str::to_owned),
            password_error: errors.first("password").map(str::to_owned),
        };
        flash.clear_old().await;
        Ok(View::new(page))
    }

    #[post("/register", name = "register.store")]
    async fn register(
        db: Db,
        flash: Flash,
        login: Login<User>,
        Validated(Form(input)): Validated<Form<RegisterForm>>,
    ) -> Result<Redirect> {
        if User::query()
            .filter(User::EMAIL.eq(&input.email))
            .exists(&db)
            .await?
        {
            flash.error("That email is already registered.").await;
            return Ok(Redirect::to("/register"));
        }
        let hash = password::hash(&input.password)?;
        let user = User::new(input.name.trim(), input.email.trim(), hash)
            .insert(&db)
            .await?;
        login.login(&user).await?;
        flash.success("Welcome!").await;
        Ok(Redirect::to("/dashboard"))
    }

    #[get("/login", name = "login")]
    async fn login_form(ctx: Ctx, csrf: Csrf, flash: Flash) -> Result<View<LoginPage>> {
        Ok(View::new(LoginPage {
            app_name: ctx.config().app.name.clone(),
            csrf_field: csrf.field().await,
            flash: flash.take().await,
            error: None,
        }))
    }

    #[post("/login", name = "login.store")]
    async fn login(
        ctx: Ctx,
        db: Db,
        csrf: Csrf,
        login: Login<User>,
        Form(input): Form<LoginForm>,
    ) -> Result<rustali::http::response::Response> {
        use rustali::http::response::IntoResponse;
        let user = User::query()
            .filter(User::EMAIL.eq(&input.email))
            .fetch_one(&db)
            .await?;
        let ok = match &user {
            Some(user) => login.attempt(user, &input.password).await?,
            None => false,
        };
        if ok {
            return Ok(Redirect::to("/dashboard").into_response());
        }
        Ok(View::new(LoginPage {
            app_name: ctx.config().app.name.clone(),
            csrf_field: csrf.field().await,
            flash: Vec::new(),
            error: Some("Those credentials do not match our records.".to_owned()),
        })
        .with_status(StatusCode::UNPROCESSABLE_ENTITY)
        .into_response())
    }

    #[post("/logout", name = "logout")]
    async fn logout(login: Login<User>, flash: Flash) -> Result<Redirect> {
        login.logout().await?;
        flash.success("Signed out.").await;
        Ok(Redirect::to("/login"))
    }

    #[get("/dashboard", name = "dashboard")]
    async fn dashboard(
        ctx: Ctx,
        csrf: Csrf,
        Auth(user): Auth<User>,
    ) -> Result<View<DashboardPage>> {
        Ok(View::new(DashboardPage {
            app_name: ctx.config().app.name.clone(),
            user_name: user.name,
            user_email: user.email,
            csrf_field: csrf.field().await,
        }))
    }

    #[get("/", name = "home")]
    async fn home(auth: OptionalAuth<User>) -> Redirect {
        if auth.is_some() {
            Redirect::to("/dashboard")
        } else {
            Redirect::to("/login")
        }
    }
}
