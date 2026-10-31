//! Pagination result.

use serde::Serialize;

/// One page of results plus the numbers a UI or API client needs.
///
/// Serialises as `{ "items": [...], "page": 2, "per_page": 20, "total": 95,
/// "last_page": 5 }`.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    /// Rows on this page.
    pub items: Vec<T>,
    /// 1-based current page.
    pub page: u64,
    /// Requested page size.
    pub per_page: u64,
    /// Total rows across all pages.
    pub total: u64,
    /// Number of the last page (at least 1).
    pub last_page: u64,
}

impl<T> Page<T> {
    /// Build a page; `page` is clamped to `>= 1` and `last_page` derived from totals.
    #[must_use]
    pub fn new(items: Vec<T>, page: u64, per_page: u64, total: u64) -> Self {
        let per_page = per_page.max(1);
        let last_page = total.div_ceil(per_page).max(1);
        Self {
            items,
            page: page.max(1),
            per_page,
            total,
            last_page,
        }
    }

    /// `true` when a next page exists.
    #[must_use]
    pub const fn has_next(&self) -> bool {
        self.page < self.last_page
    }

    /// `true` when a previous page exists.
    #[must_use]
    pub const fn has_prev(&self) -> bool {
        self.page > 1
    }

    /// Transform every item, keeping the paging numbers.
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            page: self.page,
            per_page: self.per_page,
            total: self.total,
            last_page: self.last_page,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_page_is_ceil_and_at_least_one() {
        assert_eq!(Page::<u8>::new(vec![], 1, 20, 0).last_page, 1);
        assert_eq!(Page::<u8>::new(vec![], 1, 20, 95).last_page, 5);
        assert_eq!(Page::<u8>::new(vec![], 1, 20, 100).last_page, 5);
        assert_eq!(Page::<u8>::new(vec![], 1, 20, 101).last_page, 6);
    }

    #[test]
    fn navigation_flags() {
        let page = Page::new(vec![1, 2], 2, 2, 5);
        assert!(page.has_prev());
        assert!(page.has_next());
        assert!(!Page::new(vec![1], 3, 2, 5).has_next());
        assert_eq!(page.map(|n| n * 10).items, vec![10, 20]);
    }
}
