pub const _SMALL_SPACING: f32 = 5.0;
pub const MEDIUM_SPACING: f32 = 10.0;
pub const _LARGE_SPACING: f32 = 15.0;
pub const _HUGE_SPACING: f32 = 20.0;

#[derive(Debug, Clone)]
pub enum FocusDirection {
    Up,
    Down,
}

impl FocusDirection {
    pub fn add(self, rhs: usize, length: usize) -> usize {
        match self {
            FocusDirection::Up => {
                if rhs > 0 {
                    rhs - 1
                } else {
                    length - 1
                }
            }
            FocusDirection::Down => {
                if length > 0 {
                    (rhs + 1) % length
                } else {
                    0
                }
            }
        }
    }
}
