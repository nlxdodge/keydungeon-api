use serde::{Deserialize, Serialize};

pub const DEFAULT_PAGE: u32 = 1;
pub const DEFAULT_LIMIT: u32 = 20;
pub const MAX_LIMIT: u32 = 100;

/// Raw `page`/`limit` query parameters for paged list endpoints.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl Pagination {
    /// Validate the query parameters and apply defaults.
    pub fn resolve(&self) -> Result<PageRequest, &'static str> {
        let page = self.page.unwrap_or(DEFAULT_PAGE);
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT);

        if page == 0 {
            return Err("page must be at least 1");
        }
        if limit == 0 {
            return Err("limit must be at least 1");
        }
        if limit > MAX_LIMIT {
            return Err("limit must be at most 100");
        }

        Ok(PageRequest { page, limit })
    }
}

/// A validated page request with its derived offset.
#[derive(Debug, Clone, Copy)]
pub struct PageRequest {
    pub page: u32,
    pub limit: u32,
}

impl PageRequest {
    pub fn offset(&self) -> i64 {
        i64::from(self.page - 1) * i64::from(self.limit)
    }
}

/// Envelope returned by paged list endpoints.
#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub limit: u32,
    pub total: u64,
}

impl<T> Paginated<T> {
    pub fn new(data: Vec<T>, page: PageRequest, total: u64) -> Self {
        Self {
            data,
            page: page.page,
            limit: page.limit,
            total,
        }
    }
}
