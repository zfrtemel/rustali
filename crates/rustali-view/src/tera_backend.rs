//! Optional Tera backend: runtime templates with hot reload in development.

use std::sync::Arc;

use axum::response::Html;
use rustali_core::{Error, Provider, RegisterFuture, Registry, Result};
use serde::{Deserialize, Serialize};
use tera::Tera;

/// `config/view.toml` (Tera backend).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TeraConfig {
    /// Glob of template files, relative to the crate/app root.
    pub templates: String,
}

impl Default for TeraConfig {
    fn default() -> Self {
        Self {
            templates: "views/**/*".into(),
        }
    }
}

impl TeraConfig {
    /// Section name.
    pub const SECTION: &'static str = "view";
}

/// The compiled Tera engine, registered as a service.
#[derive(Debug, Clone)]
pub struct TeraView {
    tera: Arc<Tera>,
}

impl TeraView {
    /// Compile the templates matching `glob`.
    pub fn new(glob: &str) -> Result<Self> {
        let tera = Tera::new(glob).map_err(Error::internal)?;
        Ok(Self {
            tera: Arc::new(tera),
        })
    }

    /// Render `template` with `context`, returning HTML or a framework error.
    pub fn render<T: Serialize>(&self, template: &str, context: &T) -> Result<Html<String>> {
        let ctx = tera::Context::from_serialize(context).map_err(Error::internal)?;
        let html = self.tera.render(template, &ctx).map_err(Error::internal)?;
        Ok(Html(html))
    }
}

/// Registers [`TeraView`] from `config/view.toml`.
#[derive(Debug, Default, Clone, Copy)]
pub struct TeraProvider;

impl Provider for TeraProvider {
    fn name(&self) -> &'static str {
        "view (tera)"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let config: TeraConfig = registry.config().section_or_default(TeraConfig::SECTION)?;
            let view = TeraView::new(&config.templates)?;
            registry.insert(view);
            Ok(())
        })
    }
}
