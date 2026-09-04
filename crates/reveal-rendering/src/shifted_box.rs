//! Flutter counterpart: `rendering/shifted_box.dart` (`RenderPadding`).
//!
//! `RenderShiftedBox` paint / hit-test / intrinsics wait.

use reveal_embedder::{Offset, Size};
use reveal_foundation::App;
use reveal_painting::{EdgeInsets, EdgeInsetsGeometry, TextDirection};

use crate::box_::{
    AnyRenderBox, BoxParentData, RenderBox, RenderBoxData, RenderObjectWithChildMixin,
};
use crate::object::{
    AnyRenderObject, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
};
use crate::painting_context::PaintingContext;

/// Insets its child by the given padding.
pub struct RenderPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    resolved_padding_cache: Option<EdgeInsets>,
    padding: EdgeInsetsGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderPadding {
    /// Creates a render object that insets its child.
    ///
    /// `padding` must have non-negative insets.
    pub fn new(
        app: &mut App,
        padding: EdgeInsetsGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderPadding> {
        debug_assert!(padding.is_non_negative());
        let this = RenderHandle::new_box(
            app,
            RenderPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                resolved_padding_cache: None,
                padding,
                text_direction,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The amount to pad the child in each dimension.
    pub fn padding(self: RenderHandle<Self>, app: &App) -> EdgeInsetsGeometry {
        self.get(app).padding
    }

    /// Sets [`padding`](Self::padding).
    pub fn set_padding(self: RenderHandle<Self>, app: &mut App, value: EdgeInsetsGeometry) {
        debug_assert!(value.is_non_negative());
        if self.get(app).padding == value {
            return;
        }
        self.get_mut(app).padding = value;
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`padding`](Self::padding).
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_need_resolution(app);
    }

    fn resolved_padding(&mut self) -> EdgeInsets {
        if let Some(cached) = self.resolved_padding_cache {
            return cached;
        }
        let resolved = self.padding.resolve(self.text_direction);
        debug_assert!(resolved.is_non_negative());
        self.resolved_padding_cache = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).resolved_padding_cache = None;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderPadding {
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

impl RenderObject for RenderPadding {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let padding = self.get_mut(app).resolved_padding();
        let child = self.child(app);
        if child.is_none() {
            self.set_size(
                app,
                constraints.constrain(Size::new(padding.horizontal(), padding.vertical())),
            );
            return;
        }
        let child = child.expect("checked");
        let inner_constraints = constraints.deflate(EdgeInsetsGeometry::Insets(padding));
        child.layout(app, inner_constraints, true);
        child.parent_data_of_mut::<BoxParentData>(app).offset =
            Offset::new(padding.left, padding.top);
        self.set_size(
            app,
            constraints.constrain(Size::new(
                padding.horizontal() + child.size(app).width(),
                padding.vertical() + child.size(app).height(),
            )),
        );
    }

    /// Flutter's `RenderShiftedBox.paint`.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let child_offset = child
                .as_object()
                .parent_data_of::<BoxParentData>(app)
                .offset;
            context.paint_child(app, child.as_object(), child_offset + offset);
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

impl RenderBox for RenderPadding {
    crate::render_box_accessors!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::Size;

    use crate::box_::BoxConstraints;
    use crate::proxy_box::RenderConstrainedBox;

    #[test]
    fn padding_around_tight_child() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(8.0),
            None,
            Some(child.as_box()),
        );
        padding.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(child.size(&app), Size::new(80.0, 40.0));
        assert_eq!(padding.size(&app), Size::new(96.0, 56.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(8.0, 8.0)
        );
    }

    #[test]
    fn padding_without_child() {
        let mut app = App::new();
        let padding = RenderPadding::new(&mut app, EdgeInsetsGeometry::all(10.0), None, None);
        padding.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(padding.size(&app), Size::new(100.0, 100.0));
    }
}
