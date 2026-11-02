# Migration'lar (Code-First)

Rustali'nin öne çıkan veritabanı özelliği: bir varlığı değiştir, `migrate:diff` çalıştır ve gözden
geçirilmiş bir migration al. Her migration'a yine sen sahipsin ve okursun; diff yalnızca değişenden
ilk taslağı yazar.

## Nasıl çalışır

Üç şey yer alır:

1. **Varlıkların** — `#[derive(Entity)]`, her birinin şemasını derleme zamanında kaydeder.
2. **Bir anlık görüntü** — `db/schema.snapshot.json`, şemayı en son uygulandığı hâliyle kaydeder.
   Sürüm kontrolüne commit'lenir.
3. **`migrate:diff`** — varlıkları anlık görüntüye karşı karşılaştırır, fark için bir migration
   yazar ve anlık görüntüyü günceller.

## Migration'ları etkinleştirmek

Migration'larını bir `Migrator`'da topla ve onu `.migrations::<Migrator>()` ile builder'a kaydet.
Bu, uygulamana `migrate*` komutlarını ekler.

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

Onları uygulama binary'n aracılığıyla çalıştır:

| Komut | Yaptığı |
|---|---|
| `cargo run -- migrate` | Bekleyen migration'ları uygula. |
| `cargo run -- migrate:status` | Hangi migration'ların uygulandığını göster. |
| `cargo run -- migrate:rollback` | Son grubu geri al (`--step N` ile daha fazlası). |
| `cargo run -- migrate:fresh` | Tüm tabloları bırak ve her şeyi yeniden uygula. |
| `cargo run -- migrate:refresh` | Hepsini geri al, sonra yeniden uygula. |
| `cargo run -- migrate:reset` | Her şeyi geri al. |
| `cargo run -- migrate:diff "name"` | Varlık değişikliklerinden bir migration üret. |

`fresh`, `refresh` ve `reset` yıkıcıdır; `RUSTALI_FORCE=1` ayarlamadıkça `APP_ENV=production`
altında çalışmayı reddederler.

## Code-first döngüsü

Bir varlıkta bir alanı değiştir:

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

Onu anlık görüntüye karşı diff'le:

```bash
cargo run -- migrate:diff "add published to posts"
```

```text
Detected 1 change(s):
   -  alter_posts
Wrote db/migrations/m20261101_120000_add_published_to_posts.rs
Updated db/schema.snapshot.json
```

Üretilen dosya sıradan, okunabilir `sea-orm-migration` kodudur — `add_column`'lı bir
`Table::alter()`, eşleşen bir `down` ve `DeriveIden` enum'u. Onu serbestçe gözden geçir ve düzenle;
hiçbir şey üretilen bir dizeyi çalıştırmaz.

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

Boş bir anlık görüntüden, ilk diff her varlık için tam `create_table` migration'ları yayar — yani
yeni bir projeyi önyüklemek şudur: varlıklarını tanımla, bir `migrate:diff "initial schema"`
çalıştır, gözden geçir ve `migrate`.

Her üretilen migration'ın eşleşen bir `down`'ı vardır; böylece değişiklik geri alınabilir.
Yukarıdaki kolon için, `down` onu yeniden bırakır:

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
Üretilen `down`'ı `up` kadar dikkatli gözden geçir — bir geri alma işleminin gerçek verilere karşı
çalıştıracağı şey odur.

## Tipik bir döngü

Migration'lar bağlandıktan sonraki günlük ritim:

1. Bir varlığı düzenle (bir alan ekle, bir tip değiştir).
2. `cargo run -- migrate:diff "describe the change"` — bir migration yazar ve anlık görüntüyü
   günceller.
3. **Üretilen dosyayı oku.** Diff'in tahmini tam kastettiğin şey değilse onu ayarla (bir varsayılan
   değer, bir indeks, bir veri geri doldurma).
4. Onu `src/migrations/` içine taşı ve kaydet (aşağıdaki boşluğa bak).
5. Yerel olarak uygulamak için `cargo run -- migrate`; migration'ı ve güncellenen anlık görüntüyü
   birlikte commit'le.

Üretimde, commit'lenen migration'ları gönderirsin ve dağıtımda `cargo run -- migrate`'i (ya da
binary'nin `migrate`'ini) çalıştırırsın. Üretimde `migrate:diff` **çalıştırmazsın** — diff'leme bir
geliştirme zamanı yazma adımıdır; uygulama, bir üretim veritabanına dokunan tek şeydir.

## Yıkıcı değişiklikler

Bir kolonu ya da tabloyu bırakmak, bir tip değiştirmek ya da `NULL`→`NOT NULL` sıkılaştırmak veri
kaybettirebilir; bu yüzden `migrate:diff` onları varsayılan olarak **reddeder**:

```text
1 destructive change(s) detected (drop / type change / NOT NULL). Re-run with
--allow-destructive once you have reviewed them.
```

Ne yapacağına baktıktan sonra `--allow-destructive` ile yeniden çalıştır.

## Üretilen bir migration'ı bağlamak

::: warning Bilinen boşluk
`migrate:diff`, `db/migrations/`'a yazar, ama uygulaman migration'ları `src/migrations/`'dan
derler. Şimdilik, üretilen dosyayı `src/migrations/` içine taşı, `mod m...;` satırını ekle ve
`Migrator`'ında `Box::new(m...::Migration)`'ı listele. Otomatik yerleştirme ve bağlama yol
haritasında.
:::

## Elle bir migration yazmak

Diff'i kullanmak zorunda değilsin. `rustali make:migration create_posts`, SeaORM şema builder'ıyla
(`Table::create()`, `ColumnDef`, indeksler, yabancı anahtarlar) doldurduğun boş bir migration
oluşturur. [`sea-orm-migration`]'daki her şey `rustali::migrate::prelude::*` aracılığıyla
kullanılabilir.

[`sea-orm-migration`]: https://docs.rs/sea-orm-migration

## Diff'in bugün anladığı şey

Tablolar ve kolonlar (ekle/bırak), kolon tipi değişiklikleri, null olabilirlik, benzersizlik ve
dize uzunluğu. Bilinmeyen Rust tipleri, sessizce bırakılmak yerine bir `FIXME` yorumuyla bir `TEXT`
kolonu olur; böylece bir diff asla sürpriz bir şekilde kayıplı olmaz. Struct'taki ilişkiler (düz
bir `*_id` kolonunun ötesindeki yabancı anahtarlar) gelecekteki bir iyileştirmedir.

### Yeniden adlandırmalar bir insana ihtiyaç duyar

Diff'in bir yeniden adlandırmayı bir sil-ve-oluştur'dan ayırt etme yolu yoktur: `title`'ı
`heading`'e yeniden adlandırmak ona *`title`'ı bırak, `heading` ekle* gibi görünür — ki bu eski
veriyi atar. Bu yüzden bir kolonu yeniden adlandırdığında, üretilen bırak/ekle'ye güvenme. Ya
üretilen migration'ı gerçek bir `rename_column`'a düzenle ya da o migration'ı elle yaz. Diff'in,
otomatik bir uygulama değil, *gözden geçirdiğin bir taslak* olmasının tam nedeni budur: çıkaramadığı
tek değişiklik, veriyi sessizce kaybettirecek olandır.

### Soft delete yalnızca bir kolondur

İskeletlenmiş modellerin taşıdığı `deleted_at` kolonu, diff'e göre sıradan, null olabilen bir zaman
damgasıdır — diğer herhangi bir kolon gibi migrate edilir. Soft-delete *davranışı* (`delete`'te onu
damgalamak, çöpe atılmış satırları gizlemek), migration'ın ya da şemanın bildiği bir şey değil,
çalışma zamanı bir ayardır, `.env`'deki `SOFT_DELETE`. [Temel Varlık & Soft Delete](base-entity)'e
bak.
