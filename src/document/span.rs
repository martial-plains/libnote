#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct TextSpan {
    pub start: u64,
    pub end: u64,
}

impl TextSpan {
    #[must_use]
    pub const fn contains(&self, pos: u64) -> bool {
        pos >= self.start && pos < self.end
    }

    #[must_use]
    pub const fn intersects(&self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}
