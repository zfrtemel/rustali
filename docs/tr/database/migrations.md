# Migration'lar (Code-First)

Rustali'nin en güçlü veritabanı özelliği şu: bir entity'yi değiştirirsin, `migrate:diff` çalıştırırsın
ve gözden geçirip onaylayacağın bir migration çıkar. Migration'lar yine senindir, onları sen okursun;
diff sadece değişen şeye bakıp ilk taslağı senin için yazar.

## Nasıl çalışır

İşin içinde üç şey vardır:

1. **Entity'lerin** — `#[derive(Entity)]`, her birinin şemasını derleme zamanında kaydeder.
2. **Bir anlık görüntü** — `db/schema.snapshot.json`, şemayı en son uygulandığı hâliyle saklar ve
   sürüm kontrolüne commit'lenir.
3. **`migrate:diff`** — entity'leri anlık görüntüyle karşılaştırır, aradaki fark için bir migration
   yazar ve anlık görüntüyü günceller.

## Migration'ları etkinleştirmek

Migration'larını bir `Migrator`'da topla ve onu `.migrations::<Migrator>()` ile builder'a kaydet. Bu,
uygulamana `migrate*` komutlarını kazandırır.

```rust
// src/migrations.rs
use rustali::migrate::prelude::*;

mod m20261101_000001_create_posts;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261101_000001_create_posts::Migration)]
    }
}
```

```rust
// src/main.rs
App::builder()
    .provider(DatabaseProvider)
    .migrations::<migrations::Migrator>()
    .controllers()
    .run()
    .await
```

## Komutlar

Bu komutları uygulama binary'n üzerinden çalıştırırsın:

| Komut | Yaptığı |
|---|---|
| `cargo run -- migrate` | Bekleyen migration'ları uygular. |
| `cargo run -- migrate:status` | Hangi migration'ların uygulandığını gösterir. |
| `cargo run -- migrate:rollback` | Son grubu geri alır (`--step N` ile daha fazlasını). |
| `cargo run -- migrate:fresh` | Tüm tabloları siler ve her şeyi yeniden uygular. |
| `cargo run -- migrate:refresh` | Hepsini geri alır, sonra yeniden uygular. |
| `cargo run -- migrate:reset` | Her şeyi geri alır. |
| `cargo run -- migrate:diff "name"` | Entity değişikliklerinden bir migration üretir. |

`fresh`, `refresh` ve `reset` yıkıcı komutlardır; `RUSTALI_FORCE=1` ayarlamadığın sürece
`APP_ENV=production` altında çalışmayı reddederler.

## Code-first döngüsü

Bir entity'de bir alanı değiştir:

```rust
#[derive(Entity, Clone)]
#[table("posts")]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub published: bool,   // <-- yeni kolon
    // ...
}
```

Değişikliği anlık görüntüyle diff'le:

```bash
cargo run -- migrate:diff "add published to posts"
```

```text
Detected 1 change(s):
   -  alter_posts
Wrote db/migrations/m20261101_120000_add_published_to_posts.rs
Updated db/schema.snapshot.json
```

Üretilen dosya, sıradan ve okunması kolay `sea-orm-migration` kodudur — `add_column` içeren bir
`Table::alter()`, buna karşılık gelen bir `down` ve bir `DeriveIden` enum'u. Dosyayı dilediğin gibi
gözden geçirip düzenleyebilirsin; hiçbir yerde üretilmiş bir metin olduğu gibi çalıştırılmaz.

```rust
// db/migrations/m20261101_120000_add_published_to_posts.rs (üretilmiş)
async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Posts::Table)
            .add_column(ColumnDef::new(Posts::Published).boolean().not_null())
            .to_owned())
        .await?;
    Ok(())
}
```

Boş bir anlık görüntüden başlandığında ilk diff, her entity için eksiksiz `create_table`
migration'ları üretir — yani yeni bir projeyi ayağa kaldırmak şu adımlardan ibarettir: entity'lerini
tanımla, bir `migrate:diff "initial schema"` çalıştır, gözden geçir ve `migrate` ile uygula.

Üretilen her migration'ın, değişikliğin geri alınabilmesi için buna karşılık gelen bir `down`'ı
vardır. Yukarıdaki column için `down`, o column'ı geri siler:

```rust
// "add published to posts" için üretilen down
async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Posts::Table)
            .drop_column(Posts::Published)
            .to_owned())
        .await?;
    Ok(())
}
```

`migrate:rollback` bunları çalıştırır; böylece geliştirme sırasında son grubu geri alabilirsin.
Üretilen `down`'ı en az `up` kadar dikkatli gözden geçir — çünkü bir geri alma işlemi, gerçek veriler
üzerinde tam olarak onu çalıştırır.

## Tipik bir döngü

Migration'lar bağlandıktan sonraki günlük ritim şöyle işler:

1. Bir entity'yi düzenle (bir alan ekle, bir tip değiştir).
2. `cargo run -- migrate:diff "describe the change"` — bir migration yazar ve anlık görüntüyü
   günceller.
3. **Üretilen dosyayı oku.** Diff'in çıkardığı sonuç tam olarak kastettiğin şey değilse dosyayı elden
   geçir (bir varsayılan değer, bir indeks, bir veri geri doldurması ekle).
4. Dosyayı `src/migrations/` içine taşıyıp kaydet (aşağıdaki eksiğe bak).
5. Yerelde uygulamak için `cargo run -- migrate`; ardından migration'ı ve güncellenen anlık görüntüyü
   birlikte commit'le.

Üretimde ise commit'lenmiş migration'ları yüklersin ve dağıtım sırasında `cargo run -- migrate`'i (ya
da binary'nin `migrate` komutunu) çalıştırırsın. Üretimde `migrate:diff`'i **asla çalıştırmazsın** —
diff'leme bir geliştirme zamanı yazma adımıdır; üretim veritabanına dokunan tek şey migration'ların
uygulanmasıdır.

## Yıkıcı değişiklikler

Bir column'ı ya da tabloyu silmek, bir tipi değiştirmek veya `NULL`→`NOT NULL` daraltması yapmak veri
kaybına yol açabilir; bu yüzden `migrate:diff` bu tür değişiklikleri varsayılan olarak **reddeder**:

```text
1 destructive change(s) detected (drop / type change / NOT NULL). Re-run with
--allow-destructive once you have reviewed them.
```

Ne yapacaklarını inceledikten sonra `--allow-destructive` ile yeniden çalıştır.

## Üretilen bir migration'ı bağlamak

::: warning Bilinen boşluk
`migrate:diff`, `db/migrations/` dizinine yazar; oysa uygulaman migration'ları `src/migrations/`'dan
derler. Şimdilik üretilen dosyayı `src/migrations/` içine taşı, `mod m...;` satırını ekle ve
`Migrator`'ında `Box::new(m...::Migration)`'ı listeye ekle. Dosyanın otomatik yerleştirilmesi ve
bağlanması yol haritasında yer alıyor.
:::

## Elle bir migration yazmak

Diff'i kullanmak zorunda değilsin. `rustali make:migration create_posts`, SeaORM şema builder'ıyla
(`Table::create()`, `ColumnDef`, indeksler, yabancı anahtarlar) kendi doldurduğun boş bir migration
oluşturur. [`sea-orm-migration`]'daki her şeye `rustali::migrate::prelude::*` üzerinden
erişebilirsin.

[`sea-orm-migration`]: https://docs.rs/sea-orm-migration

## Diff'in bugün anladığı şey

Tablolar ve column'lar (ekleme/silme), column tipi değişiklikleri, null olabilirlik, benzersizlik ve
metin uzunluğu. Bilinmeyen Rust tipleri sessizce atlanmak yerine, bir `FIXME` yorumuyla birlikte bir
`TEXT` column'ına dönüşür; böylece bir diff hiçbir zaman beklenmedik şekilde veri kaybına yol açmaz.
Struct'taki relation'lar (düz bir `*_id` column'ının ötesine geçen yabancı anahtarlar) ise ileride
eklenecek bir iyileştirmedir.

### Yeniden adlandırmalar bir insana ihtiyaç duyar

Diff'in, bir yeniden adlandırmayı bir sil-ve-oluştur'dan ayırt etmesi mümkün değildir: `title`'ı
`heading` olarak yeniden adlandırmak ona *`title`'ı sil, `heading` ekle* gibi görünür — ki bu da eski
veriyi silip atar. Bu yüzden bir column'ı yeniden adlandırdığında, üretilen sil/ekle çiftine güvenme.
Ya üretilen migration'ı gerçek bir `rename_column`'a dönüştür ya da o migration'ı baştan elle yaz.
Diff'in otomatik uygulanan bir işlem değil de *gözden geçirdiğin bir taslak* olmasının asıl nedeni
budur: çıkaramadığı tek değişiklik türü, tam da veriyi sessizce kaybettirecek olandır.

### Soft delete yalnızca bir column'dur

İskeletten gelen modellerin taşıdığı `deleted_at` column'ı, diff açısından sıradan ve null olabilen
bir zaman damgasından ibarettir — diğer column'lar gibi migrate edilir. Soft-delete *davranışı*
(`delete` sırasında bu alanı damgalamak, çöpe atılmış row'ları gizlemek) ise migration'ın ya da
şemanın haberdar olduğu bir şey değil, bir çalışma zamanı ayarıdır: `.env`'deki `SOFT_DELETE`. [Temel
Entity & Soft Delete](base-entity)'e bak.
