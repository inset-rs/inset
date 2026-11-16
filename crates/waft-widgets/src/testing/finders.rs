//! What a [`WidgetTester`] finds in the tree: elements and render objects by type, text, key
//! or icon, whether one is onstage, and where it is.
//!
//! Dart's tester methods take a [`Finder`](super::Finder) ([`element`](WidgetTester::element),
//! [`widget`](WidgetTester::widget), [`render_object`](WidgetTester::render_object), the `get_*`
//! geometry), through [`FindsOne`] and [`RenderTarget`], which an element, a global key and a
//! render box satisfy too.
//!
//! The eager methods left (`find_onstage_by_key`, `descendants_of_type`, `ancestor_of_type`,
//! `center_of`, `global_rect`, `render_box_of`) are older helpers that harness C's sweep moves
//! onto [`find`](super::find) and the `get_*` geometry; the onstage ones skip what Dart's finders
//! skip, by the same walk (`Element::debug_visit_onstage_children`).

use std::collections::HashSet;

use waft_foundation::App;

use super::{FindsOne, RenderTarget, WidgetTester};

impl WidgetTester {
    /// Dart's `tester.element(finder)`: the one element `target` finds.
    pub fn element(&self, target: impl FindsOne) -> crate::AnyElement {
        target.find_one(self, "element")
    }

    /// Dart's `tester.renderObject(finder)`: the render object of the one element `target`
    /// finds.
    pub fn render_object(&self, target: impl FindsOne) -> waft_rendering::AnyRenderObject {
        let element = target.find_one(self, "renderObject");
        let app = self.cell.borrow();
        element
            .find_render_object(&app)
            .expect("the widget found has a render object")
    }

    /// Dart's `tester.getSize(finder)`.
    pub fn get_size(&self, target: impl RenderTarget) -> waft_embedder::Size {
        let render = target.render_box(self, "getSize");
        render.size(&self.cell.borrow())
    }

    /// Dart's `tester.getCenter(finder)`: the global position of the centre of the one render
    /// box `target` finds.
    pub fn get_center(&self, target: impl RenderTarget) -> waft_embedder::Offset {
        self.center_for(target, "getCenter")
    }

    /// Dart's `getCenter(finder, callee: callee)`: the centre, a failure naming `callee`.
    pub(super) fn center_for(
        &self,
        target: impl RenderTarget,
        callee: &str,
    ) -> waft_embedder::Offset {
        self.global_point(target, callee, |size| {
            size.center(waft_embedder::Offset::ZERO)
        })
    }

    /// Dart's `tester.getTopLeft(finder)`.
    pub fn get_top_left(&self, target: impl RenderTarget) -> waft_embedder::Offset {
        self.global_point(target, "getTopLeft", |_| waft_embedder::Offset::ZERO)
    }

    /// Dart's `tester.getTopRight(finder)`.
    pub fn get_top_right(&self, target: impl RenderTarget) -> waft_embedder::Offset {
        self.global_point(target, "getTopRight", |size| {
            size.top_right(waft_embedder::Offset::ZERO)
        })
    }

    /// Dart's `tester.getBottomLeft(finder)`.
    pub fn get_bottom_left(&self, target: impl RenderTarget) -> waft_embedder::Offset {
        self.global_point(target, "getBottomLeft", |size| {
            size.bottom_left(waft_embedder::Offset::ZERO)
        })
    }

    /// Dart's `tester.getBottomRight(finder)`.
    pub fn get_bottom_right(&self, target: impl RenderTarget) -> waft_embedder::Offset {
        self.global_point(target, "getBottomRight", |size| {
            size.bottom_right(waft_embedder::Offset::ZERO)
        })
    }

    /// The global position of the point `at` names in the size of `target`'s render box.
    fn global_point(
        &self,
        target: impl RenderTarget,
        callee: &str,
        at: impl FnOnce(waft_embedder::Size) -> waft_embedder::Offset,
    ) -> waft_embedder::Offset {
        let render = target.render_box(self, callee);
        let app = self.cell.borrow();
        let point = at(render.size(&app));
        render.local_to_global(&app, point, None)
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

    /// Whether `element` is onstage: whether Dart's finders, walking the tree with
    /// `debugVisitOnstageChildren` (`skipOffstage: true`), reach it.
    pub fn is_onstage(&self, element: crate::AnyElement) -> bool {
        self.onstage_elements().contains(&element)
    }

    /// Every element the onstage walk reaches.
    fn onstage_elements(&self) -> HashSet<crate::AnyElement> {
        super::find::by_element_predicate("any element", |_, _| true)
            .evaluate(self)
            .into_iter()
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
        let onstage = self.onstage_elements();
        keyed
            .into_iter()
            .filter(|element| onstage.contains(element))
            .collect()
    }

    /// Reads the widget of the one element `target` finds as a `W`: Dart's
    /// `tester.widget<W>(finder)`.
    pub fn widget<W: 'static, R>(&self, target: impl FindsOne, read: impl FnOnce(&W) -> R) -> R {
        let element = target.find_one(self, "widget");
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
        super::find::by_type::<W>()
            .skip_offstage(false)
            .evaluate(self)
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
    pub fn get_rect(&self, target: impl RenderTarget) -> waft_embedder::Rect {
        let render = target.render_box(self, "getRect");
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
        let element = *super::find::text(text)
            .skip_offstage(false)
            .evaluate(self)
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
        let icon = *super::find::by_type::<crate::Icon>()
            .skip_offstage(false)
            .evaluate(self)
            .first()
            .expect("an Icon");
        self.glyph_style(icon)
    }

    /// The style of the glyph of the `Icon` `icon` is: the `RichText` below it.
    pub fn glyph_style(&self, icon: crate::AnyElement) -> waft_painting::TextStyle {
        let rich_text = super::find::by_type::<crate::RichText>()
            .skip_offstage(false)
            .evaluate(self)
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
