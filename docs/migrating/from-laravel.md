# From Laravel

Rustali deliberately echoes Laravel's vocabulary. The table maps what you know to its
Rustali equivalent; the big difference is that everything is typed and checked at
compile time.

| Laravel | Rustali |
|---|---|
| `php artisan serve` | `cargo run` |
| `php artisan make:controller` | `rustali make:controller` |
| `php artisan migrate` | `cargo run -- migrate` |
| `Route::get('/users', ...)` in `routes/web.php` | `#[get("/users")]` on the handler |
| `UserController` | `#[controller("/users")] impl UserController` |
| Middleware | tower layers; `#[middleware(...)]`, global `App::builder().layer(...)` |
| Eloquent `User::find($id)` | `User::find(&db, id).await?` |
| Eloquent model | `#[derive(Entity)]` struct (a plain POCO) |
| `$user->update([...])` | `user.update(&db).await?` |
| Migrations (hand-written) | **code-first**: `migrate:diff` generates them |
| Blade <code v-pre>{{ $var }}</code> | Askama <code v-pre>{{ var }}</code> (compile-time checked) or Tera |
| Form Request validation | `#[derive(Validate)]` + `Validated<Form<T>>` |
| `$request->validate([...])` redirect back | same behaviour, automatic |
| `Auth::user()` | `Auth<User>` extractor |
| `Gate` / Policies | `Gate` / `Policy` + `#[authorize(...)]` |
| `Hash::make` / `Hash::check` | `password::hash` / `password::verify` (argon2id) |
| CSRF `@csrf` | <code v-pre>{{ csrf.field()\|safe }}</code> |
| `session()->flash()` | `flash.success(...)` |
| Cache `Cache::remember` | `cache.remember(key, ttl, || async { ... })` |
| Mailables | `impl Mailable` + `mailer.send(...)` |
| Jobs / Queues | `#[derive(Serialize)] + impl Job`, `queue.dispatch(...)` |
| Service Providers | `impl Provider` |
| `config('app.name')` | `ctx.config().app.name` |

## What is different on purpose

- **No facades.** Services come through the typed `AppContext` / `Inject<T>`, not global
  accessors. The container is explicit (the .NET influence).
- **Errors are values.** Handlers return `Result<T>`; `?` propagates. There is no
  exception stack to catch — the error-negotiation layer renders them.
- **The compiler is your test suite for shape.** A missing template variable, a wrong
  column name, an unhandled error variant — these fail `cargo build`, not production.
