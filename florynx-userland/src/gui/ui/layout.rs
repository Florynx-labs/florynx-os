use alloc::boxed::Box;
use alloc::vec::Vec;

use super::geometry::{Constraints, Size};
use super::widget::Widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

pub fn layout_linear(
    axis: Axis,
    children: &mut Vec<Box<dyn Widget>>,
    constraints: Constraints,
    gap: i32,
) -> Size {
    let mut cursor = 0i32;
    let mut cross = 0i32;

    for child in children.iter_mut() {
        let child_constraints = match axis {
            Axis::Vertical => Constraints {
                min_w: 0,
                min_h: 0,
                max_w: constraints.max_w,
                max_h: constraints.max_h,
            },
            Axis::Horizontal => Constraints {
                min_w: 0,
                min_h: 0,
                max_w: constraints.max_w,
                max_h: constraints.max_h,
            },
        };
        let size = child.layout(child_constraints);
        match axis {
            Axis::Vertical => {
                child.set_position(0, cursor);
                cursor += size.h + gap;
                cross = cross.max(size.w);
            }
            Axis::Horizontal => {
                child.set_position(cursor, 0);
                cursor += size.w + gap;
                cross = cross.max(size.h);
            }
        }
    }

    if !children.is_empty() {
        cursor -= gap;
    }

    let wanted = match axis {
        Axis::Vertical => Size { w: cross, h: cursor },
        Axis::Horizontal => Size { w: cursor, h: cross },
    };
    constraints.clamp(wanted)
}
