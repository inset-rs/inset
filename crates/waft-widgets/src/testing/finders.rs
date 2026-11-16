//! What a [`WidgetTester`] finds in the tree: elements and render objects by type, text, key
//! or icon, whether one is onstage, and where it is.
//!
//! The finders are eager `Vec`s under Dart's names. `find_text` and `find_by_type` report
//! offstage elements too, and the `find_onstage_*` ones skip them, as Dart's finders do by
//! default (`skipOffstage: true`).

use waft_foundation::App;

use super::WidgetTester;

impl WidgetTester {
    /// The render object of `key`'s widget.
    pub fn render_object(&self, key: &crate::GlobalKey) -> waft_rendering::AnyRenderObject {
        let app = self.cell.borrow();
        key.current_context(&app)
            .expect("the keyed widget is mounted")
            .find_render_object(&app)
            .expect("the keyed widget has a render object")
    }

    /// The size of `key`'s render box: Dart's `tester.getSize(find.byKey(key))`.
    pub fn size_of(&self, key: &crate::GlobalKey) -> waft_embedder::Size {
        let render = self.render_object(key).as_box().expect("a render box");
        render.size(&self.cell.borrow())
    }

    /// Every render object of type `T` in the view, in paint order.
    pub fn all<T: waft_rendering::RenderObject>(&self) -> Vec<waft_rendering::RenderHandle<T>> {
        let mut app = self.cell.borrow_mut();
        let binding = waft_rendering::RendererBinding::instance(&mut app);
        let mut found = Vec::new();
        for render_view in binding.render_views(&app) {
            collect(&app, render_view.as_object(), &mut found);
        }
        found
    }

    /// The first render object of type `T` in the view.
    pub fn first<T: waft_rendering::RenderObject>(&self) -> waft_rendering::RenderHandle<T> {
        self.all::<T>()
            .into_iter()
            .next()
            .expect("a render object of that type")
    }

    /// The centre of `key`'s render box, in the view's coordinates.
    pub fn center_of(&self, key: &crate::GlobalKey) -> waft_embedder::Offset {
        let render = self.render_object(key).as_box().expect("a render box");
        let app = self.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, size.center(waft_embedder::Offset::ZERO), None)
    }

    /// Every mounted element, depth first with a parent before its children: the order in which
    /// Dart's finders report what they find, so `.first` and `.last` match theirs.
    pub fn elements(&self) -> Vec<crate::AnyElement> {
        let mut app = self.cell.borrow_mut();
        let root = crate::WidgetsBinding::instance(&mut app).root_element(&app);
        let mut all = Vec::new();
        let mut pending: Vec<crate::AnyElement> = root.into_iter().collect();
        while let Some(element) = pending.pop() {
            all.push(element);
            let mut children = Vec::new();
            element.visit_children(&app, &mut |child| children.push(child));
            pending.extend(children.into_iter().rev());
        }
        all
    }

    /// Dart's `find.byType(W)`: every element whose widget is a `W`.
    pub fn find_by_type<W: 'static>(&self) -> Vec<crate::AnyElement> {
        let found = self.elements();
        let app = self.cell.borrow();
        found
            .into_iter()
            .filter(|element| crate::downcast_widget::<W>(&**element.widget(&app)).is_some())
            .collect()
    }

    /// Dart's `find.text(text)`: every `Text` whose data, or its span's plain text, is `text`.
    pub fn find_text(&self, text: &str) -> Vec<crate::AnyElement> {
        let found = self.find_by_type::<crate::Text>();
        let app = self.cell.borrow();
        found
            .into_iter()
            .filter(|element| {
                let widget = crate::downcast_widget::<crate::Text>(&**element.widget(&app))
                    .expect("found by type");
                match (&widget.data, &widget.text_span) {
                    (Some(data), _) => data == text,
                    (None, Some(span)) => span.to_plain_text(true, true) == text,
                    (None, None) => false,
                }
            })
            .collect()
    }

    /// Whether `element` is onstage, as the `debugVisitOnstageChildren` overrides of the
    /// elements above it answer: in an entry each `Overlay` keeps onstage, under no `Offstage`
    /// that hides it, at each `IndexedStack`'s index, a toolbar item that is painted, and in a
    /// viewport's visible sliver and a lazy sliver's painted window. Dart's finders skip the
    /// rest (`skipOffstage: true`).
    pub fn is_onstage(&self, element: crate::AnyElement) -> bool {
        use waft_rendering::ContainerRenderObjectMixin as _;
        let app = self.cell.borrow();
        let Some(mut object) = element.find_render_object(&app) else {
            return false;
        };
        while let Some(parent) = object.parent(&app) {
            if let Some(theater) = parent.downcast::<crate::RenderTheater>(&app) {
                let mut child = theater.first_child(&app);
                for _ in 0..theater.skip_count(&app) {
                    child = child.and_then(|current| theater.child_after(&app, current));
                }
                let mut onstage = false;
                while let Some(current) = child {
                    if current.as_object() == object {
                        onstage = true;
                        break;
                    }
                    child = theater.child_after(&app, current);
                }
                if !onstage {
                    return false;
                }
            }
            if parent
                .downcast::<waft_rendering::RenderOffstage>(&app)
                .is_some_and(|offstage| offstage.offstage(&app))
            {
                return false;
            }
            // Dart's `_IndexedStackElement.debugVisitOnstageChildren`: only the child at the
            // stack's index is onstage, and none when the index is null.
            if let Some(stack) = parent.downcast::<waft_rendering::RenderIndexedStack>(&app) {
                let Some(index) = stack.index(&app) else {
                    return false;
                };
                let mut child = stack.first_child(&app);
                for _ in 0..index {
                    child = child.and_then(|current| stack.child_after(&app, current));
                }
                if child.is_none_or(|current| current.as_object() != object) {
                    return false;
                }
            }
            // Dart's `debugVisitOnstageChildren` of Material's
            // `_TextSelectionToolbarItemsLayoutElement` and Cupertino's
            // `_CupertinoTextSelectionToolbarItemsElement`: a child the items layout does not
            // paint is offstage. A single-child box that keeps its child's offset in
            // `ToolbarItemsParentData` too (Material's trailing-edge align) paints its child,
            // and the items layout always has its overflow buttons beside the items, so the rule
            // reads only a child with a sibling. Until `Element::debug_visit_onstage_children`.
            if let Some(data) = object
                .parent_data(&app)
                .and_then(|data| data.part::<crate::ToolbarItemsParentData>())
                && !data.should_paint
                && has_sibling(&app, parent)
            {
                return false;
            }
            // Dart's `_ViewportElement.debugVisitOnstageChildren`: a viewport's sliver is onstage
            // only while its geometry is visible.
            if parent.as_sliver().is_none()
                && let Some(sliver) = object.as_sliver()
                && !sliver.geometry(&app).visible
            {
                return false;
            }
            // Dart's `SliverMultiBoxAdaptorElement.debugVisitOnstageChildren`: a lazy sliver's
            // child is onstage only while its layout offset falls in the sliver's painted
            // window, which a kept-alive child's does not.
            if let Some(sliver) = parent.as_sliver()
                && let Some(child_parent_data) = object
                    .parent_data(&app)
                    .and_then(|data| data.part::<waft_rendering::SliverMultiBoxAdaptorParentData>())
            {
                let constraints = sliver.constraints(&app);
                let paint_bounds = object.paint_bounds(&app);
                let item_extent = match constraints.axis() {
                    waft_painting::Axis::Horizontal => paint_bounds.width(),
                    waft_painting::Axis::Vertical => paint_bounds.height(),
                };
                let onstage = child_parent_data
                    .layout_offset()
                    .is_some_and(|layout_offset| {
                        layout_offset
                            < constraints.scroll_offset + constraints.remaining_paint_extent
                            && layout_offset + item_extent > constraints.scroll_offset
                    });
                if !onstage {
                    return false;
                }
            }
            object = parent;
        }
        true
    }

    /// Dart's `find.text(text)` with its default `skipOffstage`: [`find_text`](Self::find_text)
    /// without the offstage ones.
    pub fn find_onstage_text(&self, text: &str) -> Vec<crate::AnyElement> {
        self.find_text(text)
            .into_iter()
            .filter(|element| self.is_onstage(*element))
            .collect()
    }

    /// Dart's `find.byType(W)` with its default `skipOffstage`:
    /// [`find_by_type`](Self::find_by_type) without the offstage ones.
    pub fn find_onstage_by_type<W: 'static>(&self) -> Vec<crate::AnyElement> {
        self.find_by_type::<W>()
            .into_iter()
            .filter(|element| self.is_onstage(*element))
            .collect()
    }

    /// Dart's `find.byKey(key)` with its default `skipOffstage`: every onstage element whose
    /// widget has `key`.
    pub fn find_onstage_by_key(&self, key: &crate::KeyRef) -> Vec<crate::AnyElement> {
        let found = self.elements();
        let keyed: Vec<crate::AnyElement> = {
            let app = self.cell.borrow();
            found
                .into_iter()
                .filter(|element| element.widget(&app).key() == Some(key))
                .collect()
        };
        keyed
            .into_iter()
            .filter(|element| self.is_onstage(*element))
            .collect()
    }

    /// Reads the widget of `element` as a `W`: Dart's `tester.widget<W>(finder)`.
    pub fn widget<W: 'static, R>(
        &self,
        element: crate::AnyElement,
        read: impl FnOnce(&W) -> R,
    ) -> R {
        let app = self.cell.borrow();
        read(
            crate::downcast_widget::<W>(&**element.widget(&app))
                .expect("the element's widget is of that type"),
        )
    }

    /// The ancestors of `element`, nearest first: Dart's `find.ancestor(of:)`.
    pub fn ancestors(&self, element: crate::AnyElement) -> Vec<crate::AnyElement> {
        let app = self.cell.borrow();
        std::iter::successors(element.parent(&app), |parent| parent.parent(&app)).collect()
    }

    /// Dart's `find.descendant(of: of, matching: find.byType(W))`: every element under `of`
    /// whose widget is a `W`.
    pub fn descendants_of_type<W: 'static>(&self, of: crate::AnyElement) -> Vec<crate::AnyElement> {
        self.find_by_type::<W>()
            .into_iter()
            .filter(|element| self.ancestors(*element).contains(&of))
            .collect()
    }

    /// The nearest ancestor of `element` whose widget is a `W`.
    pub fn ancestor_of_type<W: 'static>(&self, element: crate::AnyElement) -> crate::AnyElement {
        let ancestors = self.ancestors(element);
        let app = self.cell.borrow();
        ancestors
            .into_iter()
            .find(|ancestor| crate::downcast_widget::<W>(&**ancestor.widget(&app)).is_some())
            .expect("an ancestor of that type")
    }

    /// The render box of `element`: Dart's `tester.renderObject(finder)`.
    pub fn render_box_of(&self, element: crate::AnyElement) -> waft_rendering::AnyRenderBox {
        let app = self.cell.borrow();
        element
            .find_render_object(&app)
            .and_then(|render_object| render_object.as_box())
            .expect("the element has a render box")
    }

    /// The rectangle at `render`'s global top left with its own size, unscaled: where
    /// [`get_rect`](Self::get_rect) would differ only for a box painted scaled.
    pub fn global_rect(&self, render: waft_rendering::AnyRenderBox) -> waft_embedder::Rect {
        let app = self.cell.borrow();
        let top_left = render.local_to_global(&app, waft_embedder::Offset::ZERO, None);
        top_left & render.size(&app)
    }

    /// Dart's `tester.getRect(finder)`: the rect between the global positions of `render`'s
    /// top left and bottom right corners, so a box painted scaled has its scaled size, where
    /// [`global_rect`](Self::global_rect) keeps the box's own size.
    pub fn get_rect(&self, render: waft_rendering::AnyRenderBox) -> waft_embedder::Rect {
        let app = self.cell.borrow();
        let size = render.size(&app);
        let top_left = render.local_to_global(&app, waft_embedder::Offset::ZERO, None);
        let bottom_right =
            render.local_to_global(&app, size.bottom_right(waft_embedder::Offset::ZERO), None);
        waft_embedder::Rect::from_points(top_left, bottom_right)
    }

    /// Dart's `tester.layerListOf(finder)`: the layer of the nearest render object at or above
    /// `element`'s that has one, and every layer under it, depth first with a parent before its
    /// children.
    pub fn layer_list_of(&self, element: crate::AnyElement) -> Vec<waft_rendering::AnyLayer> {
        let app = self.cell.borrow();
        let mut current = element
            .find_render_object(&app)
            .expect("the element has a render object");
        let layer = loop {
            if let Some(layer) = current.debug_layer(&app) {
                break layer;
            }
            current = current
                .parent(&app)
                .expect("a layer above the render object");
        };
        let mut layers = Vec::new();
        walk_layers(&app, layer, &mut layers);
        layers
    }

    /// The color of the first `Text` showing `text`, as its paragraph paints it: Dart's
    /// `tester.renderObject<RenderParagraph>(find.text(text)).text.style!.color!`.
    pub fn text_color(&self, text: &str) -> waft_embedder::Color {
        let element = *self
            .find_text(text)
            .first()
            .expect("a Text showing the text");
        let render = self.render_box_of(element);
        let app = self.cell.borrow();
        let paragraph = render
            .as_object()
            .downcast::<waft_rendering::RenderParagraph>(&app)
            .expect("a Text renders a paragraph");
        paragraph
            .text(&app)
            .style()
            .and_then(|style| style.color.as_deref().copied())
            .expect("the paragraph's text has a color")
    }

    /// The style of the first `Icon`'s glyph: Dart's `tester.widget<RichText>(find.descendant(
    /// of: find.byIcon(icon), matching: find.byType(RichText))).text.style!`.
    pub fn icon_style(&self) -> waft_painting::TextStyle {
        let icon = *self.find_by_type::<crate::Icon>().first().expect("an Icon");
        self.glyph_style(icon)
    }

    /// Dart's `find.byIcon(icon)`: the first `Icon` showing `icon`.
    pub fn find_by_icon(&self, icon: &crate::IconData) -> crate::AnyElement {
        self.find_by_type::<crate::Icon>()
            .into_iter()
            .find(|element| {
                self.widget(*element, |widget: &crate::Icon| {
                    widget.icon.as_ref() == Some(icon)
                })
            })
            .expect("an Icon showing the icon")
    }

    /// The style of the glyph of the `Icon` `icon` is: the `RichText` below it.
    pub fn glyph_style(&self, icon: crate::AnyElement) -> waft_painting::TextStyle {
        let rich_text = self
            .find_by_type::<crate::RichText>()
            .into_iter()
            .find(|rich_text| self.ancestors(*rich_text).contains(&icon))
            .expect("the Icon's RichText");
        self.widget(rich_text, |rich_text: &crate::RichText| {
            rich_text
                .text
                .style()
                .cloned()
                .expect("an Icon styles its glyph")
        })
    }

    /// The cursor the mouse tracker shows for `device`: Dart's
    /// `RendererBinding.instance.mouseTracker.debugDeviceActiveCursor(device)`.
    pub fn active_cursor(&self, device: i64) -> waft_services::MouseCursorRef {
        let mut app = self.cell.borrow_mut();
        let tracker = waft_rendering::RendererBinding::instance(&mut app).mouse_tracker(&app);
        tracker
            .debug_device_active_cursor(&app, device)
            .expect("the mouse is over the view")
    }

    /// Dart's `FocusManager.instance.highlightStrategy = strategy`.
    pub fn set_highlight_strategy(&self, strategy: crate::FocusHighlightStrategy) {
        let mut app = self.cell.borrow_mut();
        crate::FocusManager::instance(&mut app).set_highlight_strategy(&mut app, strategy);
    }
}

/// Whether `parent` lays out more than one child.
fn has_sibling(app: &App, parent: waft_rendering::AnyRenderObject) -> bool {
    let mut children = 0;
    parent.visit_children(app, &mut |_| children += 1);
    children > 1
}

/// Adds `layer` and the layers under it to `layers`, depth first with a parent before its
/// children: flutter_test's `_walkLayers`.
fn walk_layers(
    app: &App,
    layer: waft_rendering::AnyContainerLayer,
    layers: &mut Vec<waft_rendering::AnyLayer>,
) {
    use waft_rendering::ErasedLayer;
    layers.push(layer.as_layer());
    let mut child = layer.first_child(app);
    while let Some(current) = child {
        match current.as_container_layer() {
            Some(container) => walk_layers(app, container, layers),
            None => layers.push(current),
        }
        child = current.next_sibling(app);
    }
}

fn collect<T: waft_rendering::RenderObject>(
    app: &App,
    node: waft_rendering::AnyRenderObject,
    found: &mut Vec<waft_rendering::RenderHandle<T>>,
) {
    if let Some(typed) = node.downcast::<T>(app) {
        found.push(typed);
    }
    node.visit_children(app, &mut |child| collect(app, child, found));
}
