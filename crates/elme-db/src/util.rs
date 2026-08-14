#[derive(Debug, Clone)]
pub struct PartialVec<T> {
    pub inner: Vec<T>,
    pub total_items: usize,
}
impl<T> PartialVec<T> {
    #[must_use]
    pub fn new(items: Vec<T>, total_items: usize) -> Self {
        Self {
            inner: items,
            total_items,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}
impl SortOrder {
    #[must_use]
    pub fn sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
    #[must_use]
    pub fn invert(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}
