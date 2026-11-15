//! How much of a view the keyboard covers, from UIKit's keyboard notices, as Flutter's
//! `KeyboardInsetManager.swift` reckons it (`shouldIgnoreKeyboardNotification`,
//! `calculateKeyboardAttachMode`, `calculateKeyboardInset`): only a docked keyboard is an inset, the
//! part of it over the view; a floating, split or undocked one, and a hidden one, cover nothing the
//! app lays out around.
//!
//! Flutter animates the inset to its target along the keyboard's own animation; this host sets the
//! target at once, so a view lays out for the keyboard's end as the keyboard starts to move. iPad's
//! Slide Over adjustment is not reckoned: an iPhone app has no Slide Over.

/// A rectangle in the screen's coordinate space, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScreenRect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl ScreenRect {
    fn max_y(self) -> f64 {
        self.y + self.height
    }

    fn is_zero(self) -> bool {
        self == ScreenRect::default()
    }

    /// No width or no height: `CGRectIsEmpty`.
    fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Where `self` and `other` overlap; empty where they do not.
    fn intersection(self, other: ScreenRect) -> ScreenRect {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = self.max_y().min(other.max_y());
        ScreenRect {
            x: left,
            y: top,
            width: (right - left).max(0.0),
            height: (bottom - top).max(0.0),
        }
    }
}

/// Which of UIKit's keyboard notices was posted: that the keyboard will show, change its frame, or
/// hide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyboardNotice {
    Show,
    ChangeFrame,
    Hide,
}

/// One keyboard notice, with what its user info says.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct KeyboardChange {
    pub(crate) notice: KeyboardNotice,
    /// Where the keyboard ends, in the screen's coordinates (`UIKeyboardFrameEndUserInfoKey`).
    pub(crate) end_frame: ScreenRect,
    /// Whether the keyboard is this app's (`UIKeyboardIsLocalUserInfoKey`), true where unsaid.
    pub(crate) is_local: bool,
}

/// How the keyboard sits, as Flutter tells it apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardMode {
    Hidden,
    Docked,
    Floating,
}

impl KeyboardChange {
    /// How many points of `view`, its bounds in the screen's coordinates, the keyboard covers once
    /// it has moved, on a screen whose bounds are `screen`; `None` for a notice to be ignored, which
    /// leaves the inset as it was.
    pub(crate) fn inset_of(self, view: ScreenRect, screen: ScreenRect) -> Option<f64> {
        if self.is_ignored() {
            return None;
        }
        let covered = match self.mode(screen) {
            KeyboardMode::Docked => self.end_frame.intersection(view).height,
            KeyboardMode::Hidden | KeyboardMode::Floating => 0.0,
        };
        Some(covered)
    }

    /// Flutter's `shouldIgnoreKeyboardNotification`: a frame change to nothing, which a dragged
    /// keyboard posts, and another app's keyboard; never a hide.
    fn is_ignored(self) -> bool {
        match self.notice {
            KeyboardNotice::Hide => false,
            KeyboardNotice::ChangeFrame if self.end_frame.is_zero() => true,
            _ if self.end_frame.is_empty() => false,
            _ => !self.is_local,
        }
    }

    /// Flutter's `calculateKeyboardAttachMode`: a keyboard whose frame is all zeroes is a shortcuts
    /// bar dropped after a drag, floating; one with no width or height is hidden; one on the screen
    /// docked if it reaches the screen's bottom and floating if it stops short of it.
    fn mode(self, screen: ScreenRect) -> KeyboardMode {
        if self.notice == KeyboardNotice::Hide {
            return KeyboardMode::Hidden;
        }
        if self.end_frame.is_zero() {
            return KeyboardMode::Floating;
        }
        if self.end_frame.is_empty() {
            return KeyboardMode::Hidden;
        }
        let on_screen = self.end_frame.intersection(screen);
        if on_screen.height.round() > 0.0 && on_screen.width > 0.0 {
            if self.end_frame.max_y().round() < screen.height {
                return KeyboardMode::Floating;
            }
            return KeyboardMode::Docked;
        }
        KeyboardMode::Hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: ScreenRect = ScreenRect {
        x: 0.0,
        y: 0.0,
        width: 393.0,
        height: 852.0,
    };

    fn change(notice: KeyboardNotice, end_frame: ScreenRect) -> KeyboardChange {
        KeyboardChange {
            notice,
            end_frame,
            is_local: true,
        }
    }

    fn keyboard_from(y: f64) -> ScreenRect {
        ScreenRect {
            x: 0.0,
            y,
            width: 393.0,
            height: SCREEN.height - y,
        }
    }

    #[test]
    fn a_docked_keyboard_covers_the_part_of_the_view_it_is_over() {
        let shown = change(KeyboardNotice::Show, keyboard_from(516.0));
        assert_eq!(shown.inset_of(SCREEN, SCREEN), Some(336.0));
        let shorter_view = ScreenRect {
            height: 700.0,
            ..SCREEN
        };
        assert_eq!(shown.inset_of(shorter_view, SCREEN), Some(184.0));
    }

    #[test]
    fn a_hidden_or_floating_keyboard_covers_nothing() {
        let hidden = change(KeyboardNotice::Hide, keyboard_from(516.0));
        assert_eq!(hidden.inset_of(SCREEN, SCREEN), Some(0.0));
        let floating = change(
            KeyboardNotice::ChangeFrame,
            ScreenRect {
                x: 20.0,
                y: 300.0,
                width: 320.0,
                height: 260.0,
            },
        );
        assert_eq!(floating.inset_of(SCREEN, SCREEN), Some(0.0));
        let dropped_bar = change(KeyboardNotice::Show, ScreenRect::default());
        assert_eq!(dropped_bar.inset_of(SCREEN, SCREEN), Some(0.0));
    }

    #[test]
    fn a_dragged_keyboard_and_another_apps_are_ignored() {
        let dragged = change(KeyboardNotice::ChangeFrame, ScreenRect::default());
        assert_eq!(dragged.inset_of(SCREEN, SCREEN), None);
        let elsewhere = KeyboardChange {
            is_local: false,
            ..change(KeyboardNotice::Show, keyboard_from(516.0))
        };
        assert_eq!(elsewhere.inset_of(SCREEN, SCREEN), None);
        let hidden_elsewhere = KeyboardChange {
            is_local: false,
            ..change(KeyboardNotice::Hide, keyboard_from(852.0))
        };
        assert_eq!(hidden_elsewhere.inset_of(SCREEN, SCREEN), Some(0.0));
    }
}
