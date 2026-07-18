#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// Output is no greater than the exact mathematical result.
    Down,
    /// Output is nearest under the provider's documented tie rule.
    Nearest,
    /// Output is no less than the exact mathematical result.
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rounded<T> {
    pub value: T,
    pub direction: Direction,
}

impl<T> Rounded<T> {
    pub const fn new(value: T, direction: Direction) -> Self {
        Self { value, direction }
    }
}
