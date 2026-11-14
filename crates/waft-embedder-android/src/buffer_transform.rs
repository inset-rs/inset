//! How SurfaceFlinger turns a buffer before it shows it: Android's `NATIVE_WINDOW_TRANSFORM_*`
//! flags, which a surface control takes for its buffer (`ASurfaceTransaction_setBufferTransform`).
//! The flags flip the buffer first, horizontally then vertically, and then turn it a quarter
//! clockwise; a half turn is both flips, and three quarters both flips and the quarter turn.

/// `NATIVE_WINDOW_TRANSFORM_FLIP_H`, `_FLIP_V` and `_ROT_90`.
const FLIP_H: i32 = 1;
const FLIP_V: i32 = 2;
const ROT_90: i32 = 4;

/// A buffer's transform, as its flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTransform(i32);

impl BufferTransform {
    /// Shown as it is.
    pub(crate) const IDENTITY: BufferTransform = BufferTransform(0);

    /// Turned `clockwise` degrees, a multiple of 90, then mirrored left to right where `mirrored`
    /// says, as a camera that looks at its user is shown.
    pub(crate) fn turned(clockwise: u32, mirrored: bool) -> BufferTransform {
        if !mirrored {
            return BufferTransform(turn_flags(clockwise));
        }
        // A mirror after the turn is a mirror before the opposite turn, and the flags flip first.
        BufferTransform(turn_flags(360 - clockwise % 360) ^ FLIP_H)
    }

    /// The flags.
    pub(crate) fn flags(self) -> i32 {
        self.0
    }

    /// Whether it turns the buffer a quarter, so its width shows as its height.
    pub(crate) fn swaps_sides(self) -> bool {
        self.0 & ROT_90 != 0
    }
}

/// The flags that turn a buffer `clockwise` degrees, to the nearest quarter below.
fn turn_flags(clockwise: u32) -> i32 {
    match clockwise % 360 / 90 {
        0 => 0,
        1 => ROT_90,
        2 => FLIP_H | FLIP_V,
        _ => FLIP_H | FLIP_V | ROT_90,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_is_androids_rotation_flags() {
        assert_eq!(BufferTransform::turned(0, false), BufferTransform::IDENTITY);
        assert_eq!(BufferTransform::turned(90, false).flags(), 4);
        assert_eq!(BufferTransform::turned(180, false).flags(), 3);
        assert_eq!(BufferTransform::turned(270, false).flags(), 7);
        assert_eq!(
            BufferTransform::turned(360, false),
            BufferTransform::IDENTITY
        );
    }

    /// Mirrored after a turn: as a camera that looks at the user is shown, each one checked as
    /// where the buffer's top-left corner lands, the flags applied in Android's order.
    #[test]
    fn a_mirror_after_the_turn_lands_the_corner_where_turning_then_mirroring_does() {
        for clockwise in [0, 90, 180, 270] {
            let transform = BufferTransform::turned(clockwise, true);
            let expected = mirror(turn([0, 0], clockwise));
            assert_eq!(apply(transform, [0, 0]), expected, "turned {clockwise}");
            assert_eq!(apply(transform, [1, 0]), mirror(turn([1, 0], clockwise)));
        }
    }

    #[test]
    fn a_quarter_turn_swaps_the_sides() {
        assert!(BufferTransform::turned(90, false).swaps_sides());
        assert!(BufferTransform::turned(270, true).swaps_sides());
        assert!(!BufferTransform::turned(180, true).swaps_sides());
    }

    /// A corner of a unit square, `[x, y]` with y down, turned `clockwise` degrees about its
    /// centre.
    fn turn([x, y]: [i32; 2], clockwise: u32) -> [i32; 2] {
        (0..clockwise / 90).fold([x, y], |[x, y], _| [1 - y, x])
    }

    fn mirror([x, y]: [i32; 2]) -> [i32; 2] {
        [1 - x, y]
    }

    /// Where `transform` puts a corner: its flips first, then its quarter turn.
    fn apply(transform: BufferTransform, corner: [i32; 2]) -> [i32; 2] {
        let flags = transform.flags();
        let [mut x, mut y] = corner;
        if flags & FLIP_H != 0 {
            x = 1 - x;
        }
        if flags & FLIP_V != 0 {
            y = 1 - y;
        }
        if flags & ROT_90 != 0 {
            [x, y] = turn([x, y], 90);
        }
        [x, y]
    }
}
