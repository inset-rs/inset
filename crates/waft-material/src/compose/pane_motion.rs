//! How each pane moves when a pane scaffold's value changes, from Compose's `PaneMotion`.
//!
//! The choice of motion is a pure function of the old value, the new value and the pane order.
//! Where a pane is drawn at a point of its motion is a pure function of that motion, the panes'
//! bounds before and after, and the spring's progress; the scaffold only drives the springs.

use waft_embedder::Rect;
use waft_physics::{Simulation, SpringDescription, SpringSimulation, Tolerance};

use crate::compose::{PaneAdaptedValue, ThreePaneScaffoldHorizontalOrder, ThreePaneScaffoldValue};

/// Compose's `PaneMotionDefaults`: the spring every pane moves on, and the delay of the delayed
/// motions as a share of the spring's own duration.
pub(crate) struct PaneMotionDefaults;

impl PaneMotionDefaults {
    const DAMPING_RATIO: f64 = 0.8;
    const STIFFNESS: f64 = 380.0;
    const DELAYED_RATIO: f64 = 0.1;

    /// A spring from 0 to 1 that ends exactly on 1.
    pub(crate) fn simulation() -> SpringSimulation {
        SpringSimulation::with_options(
            SpringDescription::with_damping_ratio_value(1.0, Self::STIFFNESS, Self::DAMPING_RATIO),
            0.0,
            1.0,
            0.0,
            true,
            Tolerance::DEFAULT_TOLERANCE,
        )
    }

    /// The same spring, held at 0 for a tenth of its duration.
    pub(crate) fn delayed_simulation() -> DelayedSimulation {
        let spring = PaneMotionDefaults::simulation();
        let delay = duration_of(&spring) * Self::DELAYED_RATIO;
        DelayedSimulation { spring, delay }
    }
}

/// The time, in seconds, after which `simulation` is done.
fn duration_of(simulation: &dyn Simulation) -> f64 {
    const STEP: f64 = 0.001;
    const LIMIT: f64 = 10.0;
    let mut time = 0.0;
    while !simulation.is_done(time) && time < LIMIT {
        time += STEP;
    }
    time
}

/// Compose's `DelayedSpringSpec`: a spring that starts after a delay.
#[derive(Debug)]
pub(crate) struct DelayedSimulation {
    spring: SpringSimulation,
    delay: f64,
}

impl Simulation for DelayedSimulation {
    fn x(&self, time: f64) -> f64 {
        self.spring.x((time - self.delay).max(0.0))
    }

    fn dx(&self, time: f64) -> f64 {
        if time <= self.delay {
            0.0
        } else {
            self.spring.dx(time - self.delay)
        }
    }

    fn is_done(&self, time: f64) -> bool {
        time > self.delay && self.spring.is_done(time - self.delay)
    }

    fn tolerance(&self) -> Tolerance {
        self.spring.tolerance()
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.spring.set_tolerance(tolerance);
    }
}

/// Whether a pane was shown before a change and is shown after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaneMotionType {
    /// Hidden before and after.
    Hidden,
    /// Shown before, hidden after.
    Exiting,
    /// Hidden before, shown after.
    Entering,
    /// Shown before and after.
    Shown,
}

impl PaneMotionType {
    fn calculate(previous_value: PaneAdaptedValue, current_value: PaneAdaptedValue) -> Self {
        let was_shown = previous_value != PaneAdaptedValue::Hidden;
        let is_shown = current_value != PaneAdaptedValue::Hidden;
        match (was_shown, is_shown) {
            (true, true) => PaneMotionType::Shown,
            (false, false) => PaneMotionType::Hidden,
            (true, false) => PaneMotionType::Exiting,
            (false, true) => PaneMotionType::Entering,
        }
    }
}

/// How one pane moves when the scaffold's value changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaneMotion {
    /// The pane does not move.
    NoMotion,
    /// A shown pane moves and resizes from its old bounds to its new ones.
    AnimateBounds,
    /// The pane slides in from the left.
    EnterFromLeft,
    /// The pane slides in from the right.
    EnterFromRight,
    /// The pane slides in from the left, after a short delay.
    EnterFromLeftDelayed,
    /// The pane slides in from the right, after a short delay.
    EnterFromRightDelayed,
    /// The pane slides out to the left.
    ExitToLeft,
    /// The pane slides out to the right.
    ExitToRight,
    /// The pane grows from its centre between two shown panes.
    EnterWithExpand,
    /// The pane shrinks to its centre between two shown panes.
    ExitWithShrink,
}

impl PaneMotion {
    pub(crate) fn motion_type(self) -> PaneMotionType {
        match self {
            PaneMotion::NoMotion => PaneMotionType::Hidden,
            PaneMotion::AnimateBounds => PaneMotionType::Shown,
            PaneMotion::EnterFromLeft
            | PaneMotion::EnterFromRight
            | PaneMotion::EnterFromLeftDelayed
            | PaneMotion::EnterFromRightDelayed
            | PaneMotion::EnterWithExpand => PaneMotionType::Entering,
            PaneMotion::ExitToLeft | PaneMotion::ExitToRight | PaneMotion::ExitWithShrink => {
                PaneMotionType::Exiting
            }
        }
    }

    fn is_delayed(self) -> bool {
        matches!(
            self,
            PaneMotion::EnterFromLeftDelayed | PaneMotion::EnterFromRightDelayed
        )
    }
}

/// Compose's `calculatePaneMotion`: each pane's motion, in `pane_order`'s order.
pub(crate) fn calculate_pane_motion(
    previous_scaffold_value: &ThreePaneScaffoldValue,
    current_scaffold_value: &ThreePaneScaffoldValue,
    pane_order: ThreePaneScaffoldHorizontalOrder,
) -> [PaneMotion; 3] {
    let roles = pane_order.roles();
    let num_of_panes = roles.len();
    let mut pane_motion_types = [PaneMotionType::Hidden; 3];
    let mut pane_motions = [PaneMotion::NoMotion; 3];
    let mut first_shown_pane_index = num_of_panes;
    let mut first_entering_pane_index = num_of_panes;
    let mut last_shown_pane_index: isize = -1;
    let mut last_entering_pane_index: isize = -1;
    for (i, role) in roles.into_iter().enumerate() {
        pane_motion_types[i] = PaneMotionType::calculate(
            previous_scaffold_value.get(role),
            current_scaffold_value.get(role),
        );
        match pane_motion_types[i] {
            PaneMotionType::Shown => {
                first_shown_pane_index = first_shown_pane_index.min(i);
                last_shown_pane_index = last_shown_pane_index.max(i as isize);
                pane_motions[i] = PaneMotion::AnimateBounds;
            }
            PaneMotionType::Entering => {
                first_entering_pane_index = first_entering_pane_index.min(i);
                last_entering_pane_index = last_entering_pane_index.max(i as isize);
            }
            PaneMotionType::Hidden | PaneMotionType::Exiting => {}
        }
    }
    let mut has_panes_exit_to_right = false;
    let mut has_panes_exit_to_left = false;
    let mut first_pane_exit_to_right_index = num_of_panes;
    let mut last_pane_exit_to_left_index: isize = -1;
    for i in 0..num_of_panes {
        let has_shown_panes_on_left = first_shown_pane_index < i;
        let has_entering_panes_on_left = first_entering_pane_index < i;
        let has_shown_panes_on_right = last_shown_pane_index > i as isize;
        let has_entering_panes_on_right = last_entering_pane_index > i as isize;
        if pane_motion_types[i] == PaneMotionType::Exiting {
            pane_motions[i] = if !has_shown_panes_on_right && !has_entering_panes_on_right {
                has_panes_exit_to_right = true;
                first_pane_exit_to_right_index = first_pane_exit_to_right_index.min(i);
                PaneMotion::ExitToRight
            } else if !has_shown_panes_on_left && !has_entering_panes_on_left {
                has_panes_exit_to_left = true;
                last_pane_exit_to_left_index = last_pane_exit_to_left_index.max(i as isize);
                PaneMotion::ExitToLeft
            } else if !has_shown_panes_on_right {
                has_panes_exit_to_right = true;
                first_pane_exit_to_right_index = first_pane_exit_to_right_index.min(i);
                PaneMotion::ExitToRight
            } else if !has_shown_panes_on_left {
                // Only showing panes on left
                has_panes_exit_to_left = true;
                last_pane_exit_to_left_index = last_pane_exit_to_left_index.max(i as isize);
                PaneMotion::ExitToLeft
            } else {
                PaneMotion::ExitWithShrink
            };
        }
    }
    for i in 0..num_of_panes {
        let has_shown_panes_on_left = first_shown_pane_index < i;
        let has_shown_panes_on_right = last_shown_pane_index > i as isize;
        let has_left_pane_exit_to_right = first_pane_exit_to_right_index < i;
        let has_right_pane_exit_to_left = last_pane_exit_to_left_index > i as isize;
        let no_blocking_panes_on_right = !has_shown_panes_on_right && !has_right_pane_exit_to_left;
        let no_blocking_panes_on_left = !has_shown_panes_on_left && !has_left_pane_exit_to_right;
        if pane_motion_types[i] == PaneMotionType::Entering {
            pane_motions[i] = if no_blocking_panes_on_right && !has_panes_exit_to_right {
                PaneMotion::EnterFromRight
            } else if no_blocking_panes_on_left && !has_panes_exit_to_left {
                PaneMotion::EnterFromLeft
            } else if no_blocking_panes_on_right {
                PaneMotion::EnterFromRightDelayed
            } else if no_blocking_panes_on_left {
                PaneMotion::EnterFromLeftDelayed
            } else {
                PaneMotion::EnterWithExpand
            };
        }
    }
    pane_motions
}

/// Compose's `PaneMotionData`: one pane's motion, where it was drawn when the motion started,
/// and where it is laid out now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaneMotionData {
    pub(crate) motion: PaneMotion,
    /// The bounds the pane was drawn at when the change started.
    pub(crate) origin: Rect,
    /// The bounds the pane is laid out at; a hidden pane keeps the last bounds it was shown at.
    pub(crate) target: Rect,
}

/// Where a pane is drawn at one point of its motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaneFrame {
    /// The bounds the pane is clipped to.
    pub(crate) bounds: Rect,
    /// The width the pane's content is laid out at, centred in `bounds`.
    pub(crate) content_width: f64,
    /// How far the content is moved right inside `bounds`.
    pub(crate) content_offset: f64,
    /// Whether the content is clipped to `bounds`: every motion but a bounds animation slides or
    /// grows the content inside fixed bounds.
    pub(crate) clips: bool,
}

impl PaneFrame {
    fn at(bounds: Rect) -> PaneFrame {
        PaneFrame {
            bounds,
            content_width: bounds.width(),
            content_offset: 0.0,
            clips: false,
        }
    }

    /// The rectangle the content covers.
    pub(crate) fn content_rect(&self) -> Rect {
        let left = self.bounds.center().dx() - self.content_width / 2.0 + self.content_offset;
        Rect::from_ltwh(
            left,
            self.bounds.top,
            self.content_width,
            self.bounds.height(),
        )
    }
}

/// Compose's `ThreePaneScaffoldMotionDataProvider`: the three panes' motion data, left to right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ThreePaneScaffoldMotionData {
    pub(crate) scaffold_width: f64,
    /// Each pane's data, left to right.
    pub(crate) panes: [PaneMotionData; 3],
}

impl ThreePaneScaffoldMotionData {
    /// Where the pane at `index` from the left is drawn, given how far the spring and the
    /// delayed spring have gone, each from 0 to 1.
    pub(crate) fn frame(&self, index: usize, progress: f64, delayed_progress: f64) -> PaneFrame {
        let data = self.panes[index];
        let progress = if data.motion.is_delayed() {
            delayed_progress
        } else {
            progress
        };
        match data.motion {
            PaneMotion::NoMotion => PaneFrame::at(data.target),
            PaneMotion::AnimateBounds => {
                PaneFrame::at(lerp_rect(data.origin, data.target, progress))
            }
            PaneMotion::EnterFromLeft | PaneMotion::EnterFromLeftDelayed => self.sliding(
                data.target,
                self.slide_in_from_left_offset() * (1.0 - progress),
            ),
            PaneMotion::EnterFromRight | PaneMotion::EnterFromRightDelayed => self.sliding(
                data.target,
                self.slide_in_from_right_offset() * (1.0 - progress),
            ),
            PaneMotion::ExitToLeft => {
                self.sliding(data.origin, self.slide_out_to_left_offset() * progress)
            }
            PaneMotion::ExitToRight => {
                self.sliding(data.origin, self.slide_out_to_right_offset() * progress)
            }
            PaneMotion::EnterWithExpand => {
                let slide =
                    (self.hidden_pane_current_left(index) - data.target.left) * (1.0 - progress);
                resizing(data.target, slide, progress)
            }
            PaneMotion::ExitWithShrink => {
                let slide = (self.hiding_pane_target_left(index) - data.origin.left) * progress;
                resizing(data.origin, slide, 1.0 - progress)
            }
        }
    }

    fn sliding(&self, bounds: Rect, content_offset: f64) -> PaneFrame {
        PaneFrame {
            bounds,
            content_width: bounds.width(),
            content_offset,
            clips: true,
        }
    }

    /// Compose's `slideInFromLeftOffset`.
    fn slide_in_from_left_offset(&self) -> f64 {
        let mut previous_pane: Option<&PaneMotionData> = None;
        for pane in self.panes.iter().rev() {
            if matches!(
                pane.motion,
                PaneMotion::EnterFromLeft | PaneMotion::EnterFromLeftDelayed
            ) {
                return -previous_pane.map_or(pane.target.right, |previous| previous.target.left);
            }
            if matches!(
                pane.motion.motion_type(),
                PaneMotionType::Shown | PaneMotionType::Entering
            ) {
                previous_pane = Some(pane);
            }
        }
        0.0
    }

    /// Compose's `slideInFromRightOffset`.
    fn slide_in_from_right_offset(&self) -> f64 {
        let mut previous_pane: Option<&PaneMotionData> = None;
        for pane in &self.panes {
            if matches!(
                pane.motion,
                PaneMotion::EnterFromRight | PaneMotion::EnterFromRightDelayed
            ) {
                return self.scaffold_width
                    - previous_pane.map_or(pane.target.left, |previous| previous.target.right);
            }
            if matches!(
                pane.motion.motion_type(),
                PaneMotionType::Shown | PaneMotionType::Entering
            ) {
                previous_pane = Some(pane);
            }
        }
        0.0
    }

    /// Compose's `slideOutToLeftOffset`.
    fn slide_out_to_left_offset(&self) -> f64 {
        let mut previous_pane: Option<&PaneMotionData> = None;
        for pane in self.panes.iter().rev() {
            if pane.motion == PaneMotion::ExitToLeft {
                return -previous_pane.map_or(pane.origin.right, |previous| previous.origin.left);
            }
            if matches!(
                pane.motion.motion_type(),
                PaneMotionType::Shown | PaneMotionType::Exiting
            ) {
                previous_pane = Some(pane);
            }
        }
        0.0
    }

    /// Compose's `slideOutToRightOffset`.
    fn slide_out_to_right_offset(&self) -> f64 {
        let mut previous_pane: Option<&PaneMotionData> = None;
        for pane in &self.panes {
            if pane.motion == PaneMotion::ExitToRight {
                return self.scaffold_width
                    - previous_pane.map_or(pane.origin.left, |previous| previous.origin.right);
            }
            if matches!(
                pane.motion.motion_type(),
                PaneMotionType::Shown | PaneMotionType::Exiting
            ) {
                previous_pane = Some(pane);
            }
        }
        0.0
    }

    /// Compose's `getHiddenPaneCurrentLeft`: the right edge of the last pane on the left that
    /// was shown.
    fn hidden_pane_current_left(&self, index: usize) -> f64 {
        let mut current_left = 0.0;
        for pane in &self.panes[..index] {
            if matches!(
                pane.motion.motion_type(),
                PaneMotionType::Shown | PaneMotionType::Exiting
            ) {
                current_left = pane.origin.right;
            }
        }
        current_left
    }

    /// Compose's `getHidingPaneTargetLeft`: the right edge of the last pane on the left that
    /// will be shown.
    fn hiding_pane_target_left(&self, index: usize) -> f64 {
        let mut target_left = 0.0;
        for pane in &self.panes[..index] {
            if matches!(
                pane.motion.motion_type(),
                PaneMotionType::Shown | PaneMotionType::Entering
            ) {
                target_left = pane.target.right;
            }
        }
        target_left
    }
}

/// Compose's `expandHorizontally` / `shrinkHorizontally` around the centre, plus a slide: the
/// content keeps its full width and shows through bounds `visible_share` as wide.
fn resizing(full: Rect, slide: f64, visible_share: f64) -> PaneFrame {
    let width = full.width() * visible_share;
    let left = full.center().dx() + slide - width / 2.0;
    PaneFrame {
        bounds: Rect::from_ltwh(left, full.top, width, full.height()),
        content_width: full.width(),
        content_offset: 0.0,
        clips: true,
    }
}

fn lerp_rect(a: Rect, b: Rect, t: f64) -> Rect {
    Rect::lerp(Some(a), Some(b), t).expect("both rects are given")
}

#[cfg(test)]
mod tests {
    use super::PaneMotion::*;
    use super::*;
    use crate::compose::PaneAdaptedValue::{Expanded, Hidden};

    fn value(index: usize) -> ThreePaneScaffoldValue {
        let (primary, secondary, tertiary) = match index {
            0 => (Hidden, Hidden, Hidden),
            1 => (Expanded, Hidden, Hidden),
            2 => (Hidden, Expanded, Hidden),
            3 => (Hidden, Hidden, Expanded),
            4 => (Expanded, Expanded, Hidden),
            5 => (Expanded, Hidden, Expanded),
            6 => (Hidden, Expanded, Expanded),
            _ => (Expanded, Expanded, Expanded),
        };
        ThreePaneScaffoldValue {
            primary,
            secondary,
            tertiary,
        }
    }

    const ORDER: ThreePaneScaffoldHorizontalOrder = ThreePaneScaffoldHorizontalOrder::SUPPORTING;

    /// Compose's `PaneMotionTest.ExpectedThreePaneMotions`: from each value, to each value.
    const EXPECTED: [[[PaneMotion; 3]; 8]; 8] = [
        [
            [NoMotion, NoMotion, NoMotion],
            [EnterFromRight, NoMotion, NoMotion],
            [NoMotion, EnterFromRight, NoMotion],
            [NoMotion, NoMotion, EnterFromRight],
            [EnterFromRight, EnterFromRight, NoMotion],
            [EnterFromRight, NoMotion, EnterFromRight],
            [NoMotion, EnterFromRight, EnterFromRight],
            [EnterFromRight, EnterFromRight, EnterFromRight],
        ],
        [
            [ExitToRight, NoMotion, NoMotion],
            [AnimateBounds, NoMotion, NoMotion],
            [ExitToLeft, EnterFromRight, NoMotion],
            [ExitToLeft, NoMotion, EnterFromRight],
            [AnimateBounds, EnterFromRight, NoMotion],
            [AnimateBounds, NoMotion, EnterFromRight],
            [ExitToLeft, EnterFromRight, EnterFromRight],
            [AnimateBounds, EnterFromRight, EnterFromRight],
        ],
        [
            [NoMotion, ExitToRight, NoMotion],
            [EnterFromLeft, ExitToRight, NoMotion],
            [NoMotion, AnimateBounds, NoMotion],
            [NoMotion, ExitToLeft, EnterFromRight],
            [EnterFromLeft, AnimateBounds, NoMotion],
            [EnterFromLeft, ExitToRight, EnterFromRightDelayed],
            [NoMotion, AnimateBounds, EnterFromRight],
            [EnterFromLeft, AnimateBounds, EnterFromRight],
        ],
        [
            [NoMotion, NoMotion, ExitToRight],
            [EnterFromLeft, NoMotion, ExitToRight],
            [NoMotion, EnterFromLeft, ExitToRight],
            [NoMotion, NoMotion, AnimateBounds],
            [EnterFromLeft, EnterFromLeft, ExitToRight],
            [EnterFromLeft, NoMotion, AnimateBounds],
            [NoMotion, EnterFromLeft, AnimateBounds],
            [EnterFromLeft, EnterFromLeft, AnimateBounds],
        ],
        [
            [ExitToRight, ExitToRight, NoMotion],
            [AnimateBounds, ExitToRight, NoMotion],
            [ExitToLeft, AnimateBounds, NoMotion],
            [ExitToLeft, ExitToLeft, EnterFromRight],
            [AnimateBounds, AnimateBounds, NoMotion],
            [AnimateBounds, ExitToRight, EnterFromRightDelayed],
            [ExitToLeft, AnimateBounds, EnterFromRight],
            [AnimateBounds, AnimateBounds, EnterFromRight],
        ],
        [
            [ExitToRight, NoMotion, ExitToRight],
            [AnimateBounds, NoMotion, ExitToRight],
            [ExitToLeft, EnterFromRightDelayed, ExitToRight],
            [ExitToLeft, NoMotion, AnimateBounds],
            [AnimateBounds, EnterFromRightDelayed, ExitToRight],
            [AnimateBounds, NoMotion, AnimateBounds],
            [ExitToLeft, EnterFromLeftDelayed, AnimateBounds],
            [AnimateBounds, EnterWithExpand, AnimateBounds],
        ],
        [
            [NoMotion, ExitToRight, ExitToRight],
            [EnterFromLeft, ExitToRight, ExitToRight],
            [NoMotion, AnimateBounds, ExitToRight],
            [NoMotion, ExitToLeft, AnimateBounds],
            [EnterFromLeft, AnimateBounds, ExitToRight],
            [EnterFromLeftDelayed, ExitToLeft, AnimateBounds],
            [NoMotion, AnimateBounds, AnimateBounds],
            [EnterFromLeft, AnimateBounds, AnimateBounds],
        ],
        [
            [ExitToRight, ExitToRight, ExitToRight],
            [AnimateBounds, ExitToRight, ExitToRight],
            [ExitToLeft, AnimateBounds, ExitToRight],
            [ExitToLeft, ExitToLeft, AnimateBounds],
            [AnimateBounds, AnimateBounds, ExitToRight],
            [AnimateBounds, ExitWithShrink, AnimateBounds],
            [ExitToLeft, AnimateBounds, AnimateBounds],
            [AnimateBounds, AnimateBounds, AnimateBounds],
        ],
    ];

    #[test]
    fn every_change_between_two_values_picks_compose_s_motions() {
        for (from, row) in EXPECTED.iter().enumerate() {
            for (to, expected) in row.iter().enumerate() {
                assert_eq!(
                    calculate_pane_motion(&value(from), &value(to), ORDER),
                    *expected,
                    "from {:?} to {:?}",
                    value(from),
                    value(to)
                );
            }
        }
    }

    fn data(motion: PaneMotion, origin: Rect, target: Rect) -> PaneMotionData {
        PaneMotionData {
            motion,
            origin,
            target,
        }
    }

    /// A list (0 to 360) beside a detail (384 to 1000) on a 1000-wide scaffold.
    fn list_detail_motion(list: PaneMotion, detail: PaneMotion) -> ThreePaneScaffoldMotionData {
        let list_bounds = Rect::from_ltwh(0.0, 0.0, 360.0, 600.0);
        let detail_bounds = Rect::from_ltwh(384.0, 0.0, 616.0, 600.0);
        ThreePaneScaffoldMotionData {
            scaffold_width: 1000.0,
            panes: [
                data(list, list_bounds, list_bounds),
                data(detail, detail_bounds, detail_bounds),
                data(NoMotion, Rect::ZERO, Rect::ZERO),
            ],
        }
    }

    #[test]
    fn an_entering_pane_slides_in_from_beside_its_neighbour() {
        let motion = list_detail_motion(AnimateBounds, EnterFromRight);
        let start = motion.frame(1, 0.0, 0.0);
        assert_eq!(start.bounds, Rect::from_ltwh(384.0, 0.0, 616.0, 600.0));
        assert_eq!(start.content_offset, 640.0);
        assert!(start.clips);
        assert_eq!(motion.frame(1, 1.0, 1.0).content_offset, 0.0);
    }

    #[test]
    fn an_exiting_pane_slides_out_to_the_scaffold_edge() {
        let motion = list_detail_motion(ExitToLeft, AnimateBounds);
        assert_eq!(motion.frame(0, 0.0, 0.0).content_offset, 0.0);
        assert_eq!(motion.frame(0, 1.0, 1.0).content_offset, -384.0);
    }

    #[test]
    fn a_delayed_pane_follows_the_delayed_spring() {
        let motion = list_detail_motion(AnimateBounds, EnterFromRightDelayed);
        assert_eq!(motion.frame(1, 1.0, 0.0).content_offset, 640.0);
    }

    #[test]
    fn a_pane_between_two_shown_panes_grows_from_its_centre() {
        let target = Rect::from_ltwh(400.0, 0.0, 200.0, 600.0);
        let motion = ThreePaneScaffoldMotionData {
            scaffold_width: 1000.0,
            panes: [
                data(
                    AnimateBounds,
                    Rect::from_ltwh(0.0, 0.0, 300.0, 600.0),
                    Rect::ZERO,
                ),
                data(EnterWithExpand, target, target),
                data(AnimateBounds, Rect::ZERO, Rect::ZERO),
            ],
        };
        let start = motion.frame(1, 0.0, 0.0);
        assert_eq!(start.bounds.width(), 0.0);
        assert_eq!(start.content_width, 200.0);
        assert_eq!(start.bounds.center().dx(), 400.0);
        let end = motion.frame(1, 1.0, 1.0);
        assert_eq!(end.bounds, target);
        assert_eq!(end.content_rect(), target);
    }

    #[test]
    fn the_delayed_spring_waits_then_moves_on_the_same_spring() {
        let spring = PaneMotionDefaults::simulation();
        let delayed = PaneMotionDefaults::delayed_simulation();
        assert!(delayed.delay > 0.0);
        assert_eq!(delayed.x(delayed.delay / 2.0), 0.0);
        assert_eq!(delayed.x(delayed.delay + 0.1), spring.x(0.1));
    }

    #[test]
    fn the_spring_overshoots_and_settles_on_its_end() {
        let spring = PaneMotionDefaults::simulation();
        let peak = (1..1000)
            .map(|ms| spring.x(ms as f64 / 1000.0))
            .fold(0.0, f64::max);
        assert!(peak > 1.0, "a damping ratio of 0.8 overshoots, peak {peak}");
        assert_eq!(spring.x(5.0), 1.0);
    }
}
