//! Offset (page/limit) pagination for admin-style listings.
//!
//! For large, append-heavy feeds (notifications, audit logs, shipment events)
//! prefer keyset/cursor pagination on `(created_at, id)`; add a `CursorParams`
//! type here when the first such endpoint is built.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub const DEFAULT_PAGE_SIZE: u32 = 20;
pub const MAX_PAGE_SIZE: u32 = 100;

/// Raw query parameters. Always convert with [`PageParams::resolve`].
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PageParams {
    /// 1-based page number.
    pub page: Option<u32>,
    /// Items per page (max 100).
    pub limit: Option<u32>,
}

impl PageParams {
    pub fn resolve(&self) -> PageRequest {
        PageRequest {
            page: self.page.unwrap_or(1).max(1),
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, MAX_PAGE_SIZE),
        }
    }
}

/// Sanitised pagination request, safe to pass to repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    page: u32,
    limit: u32,
}

impl PageRequest {
    pub fn limit(self) -> i64 {
        i64::from(self.limit)
    }

    pub fn offset(self) -> i64 {
        i64::from(self.page - 1) * i64::from(self.limit)
    }

    pub fn meta(self, total_items: i64) -> PageMeta {
        let total_items = u64::try_from(total_items).unwrap_or(0);
        PageMeta {
            page: self.page,
            limit: self.limit,
            total_items,
            total_pages: total_items.div_ceil(u64::from(self.limit)),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PageMeta {
    pub page: u32,
    pub limit: u32,
    pub total_items: u64,
    pub total_pages: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_limit_and_page() {
        let request = PageParams {
            page: Some(0),
            limit: Some(10_000),
        }
        .resolve();
        assert_eq!(request.limit(), i64::from(MAX_PAGE_SIZE));
        assert_eq!(request.offset(), 0);
    }

    #[test]
    fn computes_offset_and_meta() {
        let request = PageParams {
            page: Some(3),
            limit: Some(25),
        }
        .resolve();
        assert_eq!(request.offset(), 50);

        let meta = request.meta(51);
        assert_eq!(meta.total_pages, 3);
    }
}
