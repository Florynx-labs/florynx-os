#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.w && p.y < self.y + self.h
    }

    pub fn union(&self, rhs: Rect) -> Rect {
        let x1 = self.x.min(rhs.x);
        let y1 = self.y.min(rhs.y);
        let x2 = (self.x + self.w).max(rhs.x + rhs.w);
        let y2 = (self.y + self.h).max(rhs.y + rhs.h);
        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constraints {
    pub min_w: i32,
    pub min_h: i32,
    pub max_w: i32,
    pub max_h: i32,
}

impl Constraints {
    pub fn tight(size: Size) -> Self {
        Self {
            min_w: size.w,
            min_h: size.h,
            max_w: size.w,
            max_h: size.h,
        }
    }

    pub fn loosen(&self) -> Self {
        Self {
            min_w: 0,
            min_h: 0,
            max_w: self.max_w,
            max_h: self.max_h,
        }
    }

    pub fn clamp(&self, requested: Size) -> Size {
        Size {
            w: requested.w.max(self.min_w).min(self.max_w),
            h: requested.h.max(self.min_h).min(self.max_h),
        }
    }
}
