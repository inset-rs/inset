//! Flutter counterpart: `rendering/proxy_box.dart` (`HitTestBehavior`,
//! `RenderConstrainedBox`).
//!
//! `RenderProxyBox` paint / hit-test / intrinsics wait.

use reveal_embedder::Size;
use reveal_foundation::App;

use crate::box_::{
    AnyRenderBox, BoxConstraints, RenderBox, RenderBoxData, RenderObjectWithChildMixin,
};
use crate::object::{
    AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData,
};

/// How to behave during hit tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestBehavior {
    /// Targets that defer to their children receive events within their bounds
    /// only if one of their children is hit by the hit test.
    DeferToChild,

    /// Opaque targets can be hit by hit tests, causing them to both receive
    /// events within their bounds and prevent targets visually behind them from
    /// also receiving events.
    Opaque,

    /// Translucent targets both receive events within their bounds and permit
    /// targets visually behind them to also receive events.
    Translucent,
}

/// Imposes additional constraints on its child.
pub struct RenderConstrainedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    additional_constraints: BoxConstraints,
}

impl RenderConstrainedBox {
    /// Creates a render box that constrains its child.
    ///
    /// `additional_constraints` must be valid.
    pub fn new(
        app: &mut App,
        additional_constraints: BoxConstraints,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderConstrainedBox> {
        debug_assert!(additional_constraints.debug_assert_is_valid(false));
        let this = RenderHandle::new_box(
            app,
            RenderConstrainedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                additional_constraints,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Additional constraints to apply to the child during layout.
    pub fn additional_constraints(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        self.get(app).additional_constraints
    }

    /// Sets [`additional_constraints`](Self::additional_constraints).
    pub fn set_additional_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        value: BoxConstraints,
    ) {
        debug_assert!(value.debug_assert_is_valid(false));
        if self.get(app).additional_constraints == value {
            return;
        }
        self.get_mut(app).additional_constraints = value;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderConstrainedBox {
    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderObject for RenderConstrainedBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let additional = self.get(app).additional_constraints;
        if let Some(child) = self.child(app) {
            child.layout(app, additional.enforce(constraints), true);
            self.set_size(app, child.size(app));
        } else {
            self.set_size(app, additional.enforce(constraints).constrain(Size::ZERO));
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderConstrainedBox {
    crate::render_box_accessors!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_::BoxParentData;

    #[test]
    fn constrained_box_without_child() {
        let mut app = App::new();
        let box_ =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(box_.size(&app), Size::new(80.0, 40.0));
    }

    #[test]
    fn additional_constraints_change_marks_layout() {
        let mut app = App::new();
        let box_ =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert!(!box_.debug_needs_layout(&app));
        box_.set_additional_constraints(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)));
        assert!(box_.debug_needs_layout(&app));
        box_.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(box_.size(&app), Size::new(20.0, 20.0));
    }

    /// Flutter's `RenderBox.setupParentData` installs `BoxParentData` on every box child.
    #[test]
    fn constrained_box_child_gets_box_parent_data() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        let parent = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight(Size::new(10.0, 10.0)),
            Some(child.as_box()),
        );
        parent.layout(&mut app, BoxConstraints::new(), false);
        assert!(child.as_box().parent_data_is::<BoxParentData>(&app));
    }
}
