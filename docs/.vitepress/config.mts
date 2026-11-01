import { defineConfig } from 'vitepress'

// VitePress configuration for the Rustali documentation site.
// Dev:   npm run docs:dev     Build: npm run docs:build
export default defineConfig({
  title: 'Rustali',
  description: "Laravel's developer experience, .NET-grade engineering, in Rust.",
  lastUpdated: true,
  cleanUrls: true,
  ignoreDeadLinks: true,

  themeConfig: {
    search: { provider: 'local' },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/zfrtemel/rustali' },
    ],
    nav: [
      { text: 'Guide', link: '/getting-started/first-app' },
      { text: 'Reference', link: '/migrating/from-laravel' },
      {
        text: 'crates.io',
        link: 'https://crates.io/crates/rustali',
      },
    ],
    sidebar: sidebar('en'),
  },

  locales: {
    root: { label: 'English', lang: 'en' },
    tr: {
      label: 'Türkçe',
      lang: 'tr',
      link: '/tr/',
      themeConfig: {
        nav: [
          { text: 'Rehber', link: '/tr/getting-started/first-app' },
          { text: 'Referans', link: '/tr/migrating/from-laravel' },
        ],
        sidebar: sidebar('tr'),
      },
    },
  },
})

// One sidebar definition, prefixed per locale.
function sidebar(locale: 'en' | 'tr') {
  const p = locale === 'tr' ? '/tr' : ''
  const t = (en: string, tr: string) => (locale === 'tr' ? tr : en)
  return [
    {
      text: t('Getting Started', 'Başlangıç'),
      collapsed: false,
      items: [
        { text: t('Installation', 'Kurulum'), link: `${p}/getting-started/installation` },
        { text: t('Your First App', 'İlk Uygulaman'), link: `${p}/getting-started/first-app` },
        { text: t('Project Structure', 'Proje Yapısı'), link: `${p}/getting-started/structure` },
        { text: t('Configuration', 'Yapılandırma'), link: `${p}/getting-started/configuration` },
      ],
    },
    {
      text: t('The Basics', 'Temeller'),
      collapsed: false,
      items: [
        { text: t('Routing & Controllers', 'Yönlendirme & Controller'), link: `${p}/basics/routing` },
        { text: t('Middleware & Filters', 'Middleware & Filtreler'), link: `${p}/basics/middleware` },
        { text: t('Requests & Validation', 'İstekler & Doğrulama'), link: `${p}/basics/validation` },
        { text: t('Views', 'Görünümler'), link: `${p}/basics/views` },
        { text: t('Responses & Errors', 'Yanıtlar & Hatalar'), link: `${p}/basics/responses` },
      ],
    },
    {
      text: t('The Database', 'Veritabanı'),
      collapsed: false,
      items: [
        { text: t('Entities', 'Varlıklar (Entity)'), link: `${p}/database/entities` },
        { text: t('Model Methods', 'Model Metotları'), link: `${p}/database/model-methods` },
        { text: t('Queries', 'Sorgular'), link: `${p}/database/queries` },
        { text: t('Pagination', 'Sayfalama'), link: `${p}/database/pagination` },
        {
          text: t('Relationships', 'İlişkiler'),
          collapsed: true,
          items: [
            { text: t('Overview', 'Genel Bakış'), link: `${p}/database/relations/` },
            { text: 'has_many', link: `${p}/database/relations/has-many` },
            { text: 'belongs_to', link: `${p}/database/relations/belongs-to` },
            { text: 'has_one', link: `${p}/database/relations/has-one` },
            { text: 'many_to_many', link: `${p}/database/relations/many-to-many` },
          ],
        },
        { text: t('Migrations (Code-First)', 'Migration (Code-First)'), link: `${p}/database/migrations` },
      ],
    },
    {
      text: t('Security', 'Güvenlik'),
      collapsed: false,
      items: [
        { text: t('Authentication', 'Kimlik Doğrulama'), link: `${p}/security/authentication` },
        { text: t('Authorization', 'Yetkilendirme'), link: `${p}/security/authorization` },
        { text: t('CSRF & Sessions', 'CSRF & Oturumlar'), link: `${p}/security/csrf` },
      ],
    },
    {
      text: t('Digging Deeper', 'Derinlemesine'),
      collapsed: false,
      items: [
        { text: t('Container & Providers', 'Konteyner & Provider'), link: `${p}/deeper/providers` },
        { text: t('Console Commands', 'Konsol Komutları'), link: `${p}/deeper/console` },
        { text: t('Cache', 'Önbellek'), link: `${p}/deeper/cache` },
        { text: t('Mail', 'E-posta'), link: `${p}/deeper/mail` },
        { text: t('Queues & Jobs', 'Kuyruklar & İşler'), link: `${p}/deeper/queues` },
        { text: t('Testing', 'Test'), link: `${p}/deeper/testing` },
      ],
    },
    {
      text: t('Reference', 'Referans'),
      collapsed: false,
      items: [
        { text: t('From Laravel', 'Laravel’den Geçiş'), link: `${p}/migrating/from-laravel` },
      ],
    },
  ]
}
