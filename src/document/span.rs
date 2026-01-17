#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    #[must_use]
    pub const fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    #[must_use]
    pub const fn intersects(&self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}
