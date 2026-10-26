use super::track::Track;

/// A library category. Empty categories stay in the library.
#[derive(Debug, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub created_at: i64,
}

/// One category and the tracks that belong to it, in screen order.
#[derive(Debug, Clone)]
pub struct CategoryWithTracks {
    pub category: Category,
    pub tracks: Vec<Track>,
}
