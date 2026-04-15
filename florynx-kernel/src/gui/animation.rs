// =============================================================================
// Florynx Kernel — Animation System
// =============================================================================
// Generic LERP-based animation engine. Every animated property transitions
// smoothly from current → target. Updated once per frame.
// =============================================================================

/// Linear interpolation between two values.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Ease-out cubic: decelerates smoothly (feels natural for UI).
#[inline]
pub fn ease_out(t: f32) -> f32 {
    let t1 = 1.0 - t;
    1.0 - t1 * t1 * t1
}

/// Ease-in-out: smooth start and stop.
#[inline]
pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let v = -2.0 * t + 2.0;
        1.0 - (v * v * v) / 2.0
    }
}

// ---------------------------------------------------------------------------
// Single-axis animation
// ---------------------------------------------------------------------------

/// Threshold below which we snap to target (prevents infinite asymptotic approach).
const SNAP_THRESHOLD: f32 = 0.005;

#[derive(Debug, Clone, Copy)]
pub struct Animation {
    pub current: f32,
    pub target: f32,
    /// Interpolation speed per frame (0.0 = frozen, 1.0 = instant). Typical: 0.08–0.25.
    pub speed: f32,
}

impl Animation {
    pub const fn new(value: f32, speed: f32) -> Self {
        Animation {
            current: value,
            target: value,
            speed,
        }
    }

    /// Advance one frame. Returns true if the value changed (needs redraw).
    #[inline]
    pub fn tick(&mut self) -> bool {
        let diff = self.target - self.current;
        if diff.abs() < SNAP_THRESHOLD {
            if (self.current - self.target).abs() > 0.001 {
                self.current = self.target;
                return true;
            }
            return false;
        }
        self.current = lerp(self.current, self.target, self.speed);
        true
    }

    /// Set a new target. Returns true if target actually changed.
    #[inline]
    pub fn set_target(&mut self, target: f32) -> bool {
        if (self.target - target).abs() > 0.001 {
            self.target = target;
            true
        } else {
            false
        }
    }

    /// Snap immediately to a value (no animation).
    #[inline]
    pub fn snap(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// True if animation is still in progress.
    #[inline]
    pub fn is_animating(&self) -> bool {
        (self.current - self.target).abs() >= SNAP_THRESHOLD
    }

    /// Current value as integer (for pixel coordinates).
    #[inline]
    pub fn as_usize(&self) -> usize {
        self.current.max(0.0) as usize
    }
}

// ---------------------------------------------------------------------------
// 2D position animation (convenience wrapper)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct AnimatedPos {
    pub x: Animation,
    pub y: Animation,
}

impl AnimatedPos {
    pub const fn new(x: f32, y: f32, speed: f32) -> Self {
        AnimatedPos {
            x: Animation::new(x, speed),
            y: Animation::new(y, speed),
        }
    }

    /// Advance both axes. Returns true if either changed.
    #[inline]
    pub fn tick(&mut self) -> bool {
        let cx = self.x.tick();
        let cy = self.y.tick();
        cx || cy
    }

    pub fn set_target(&mut self, x: f32, y: f32) {
        self.x.set_target(x);
        self.y.set_target(y);
    }

    pub fn snap(&mut self, x: f32, y: f32) {
        self.x.snap(x);
        self.y.snap(y);
    }

    pub fn is_animating(&self) -> bool {
        self.x.is_animating() || self.y.is_animating()
    }
}

// ---------------------------------------------------------------------------
// Opacity animation (0.0 = transparent, 1.0 = opaque)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct AnimatedOpacity {
    pub opacity: Animation,
}

impl AnimatedOpacity {
    pub const fn new(initial: f32, speed: f32) -> Self {
        AnimatedOpacity {
            opacity: Animation {
                current: initial,
                target: initial,
                speed,
            },
        }
    }

    pub fn tick(&mut self) -> bool {
        self.opacity.tick()
    }

    pub fn fade_in(&mut self) {
        self.opacity.set_target(1.0);
    }

    pub fn fade_out(&mut self) {
        self.opacity.set_target(0.0);
    }

    /// Alpha as u8 (0–255).
    pub fn alpha(&self) -> u8 {
        (self.opacity.current.clamp(0.0, 1.0) * 255.0) as u8
    }
}

// ---------------------------------------------------------------------------
// Scale animation (for dock hover effect)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct AnimatedScale {
    pub scale: Animation,
}

impl AnimatedScale {
    pub const fn new(initial: f32, speed: f32) -> Self {
        AnimatedScale {
            scale: Animation {
                current: initial,
                target: initial,
                speed,
            },
        }
    }

    pub fn tick(&mut self) -> bool {
        self.scale.tick()
    }

    pub fn set_target(&mut self, target: f32) {
        self.scale.set_target(target);
    }

    /// Compute a scaled size from a base size.
    pub fn apply(&self, base: usize) -> usize {
        (base as f32 * self.scale.current) as usize
    }
}

// ---------------------------------------------------------------------------
// Size animation (for smooth window resize)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct AnimatedSize {
    pub w: Animation,
    pub h: Animation,
}

impl AnimatedSize {
    pub const fn new(w: f32, h: f32, speed: f32) -> Self {
        AnimatedSize {
            w: Animation::new(w, speed),
            h: Animation::new(h, speed),
        }
    }

    /// Advance both axes. Returns true if either changed.
    #[inline]
    pub fn tick(&mut self) -> bool {
        let cw = self.w.tick();
        let ch = self.h.tick();
        cw || ch
    }

    pub fn set_target(&mut self, w: f32, h: f32) {
        self.w.set_target(w);
        self.h.set_target(h);
    }

    pub fn snap(&mut self, w: f32, h: f32) {
        self.w.snap(w);
        self.h.snap(h);
    }

    pub fn is_animating(&self) -> bool {
        self.w.is_animating() || self.h.is_animating()
    }
}
