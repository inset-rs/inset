//! Flutter counterpart: `rendering/layer.dart` (`PictureLayer`, `ContainerLayer`, `OffsetLayer`,
//! `OpacityLayer`, `TransformLayer`, `ClipRectLayer`, `ClipRRectLayer`, `ClipPathLayer`,
//! `AnnotatedRegionLayer`, `AnnotationEntry`, `AnnotationResult`, `Layer.find` /
//! `findAllAnnotations`).
//!
//! Layers are data, not objects. A repaint boundary retains its recording as [`PaintItem`]s and
//! composites it through one [`CompositedLayer`]; the host composes the frame from those retained
//! pieces each time. See `PORTING.md`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::sync::Arc;

use std::sync::atomic::{AtomicU64, Ordering};

use reveal_embedder::{
    Backdrop, BlendMode, Canvas, Clip, ClipOp, Color, FillRule, ImageFilter, Matrix4, Offset,
    Paint, Path, Picture, RRect, Rect, Size, rrect_radii_elliptical,
};
use reveal_foundation::App;

use crate::debug::debug_disable_opacity_layers;
use crate::object::AnyRenderObject;

/// Information collected for an annotation that is found in the layer tree.
///
/// See also:
///
///  * [`BoundaryLayer::find_all_annotations`], which creates and uses objects of this type.
#[derive(Debug)]
pub struct AnnotationEntry<T> {
    /// The annotation object that is found.
    pub annotation: Rc<T>,
    /// The target location described by the local coordinate space of the annotation object.
    pub local_position: Offset,
}

impl<T> AnnotationEntry<T> {
    /// Create an entry of found annotation by providing the object and related information.
    pub fn new(annotation: Rc<T>, local_position: Offset) -> AnnotationEntry<T> {
        AnnotationEntry {
            annotation,
            local_position,
        }
    }
}

/// Information collected about a list of annotations that are found in the layer tree.
///
/// See also:
///
///  * [`AnnotationEntry`], which are members of this type.
///  * [`BoundaryLayer::find_all_annotations`], which creates and uses an object of this type.
#[derive(Debug)]
pub struct AnnotationResult<T> {
    entries: Vec<AnnotationEntry<T>>,
}

impl<T> AnnotationResult<T> {
    /// An empty result.
    pub fn new() -> AnnotationResult<T> {
        AnnotationResult {
            entries: Vec::new(),
        }
    }

    /// Add a new entry to the end of the result.
    ///
    /// Usually, entries should be added in order from most specific to least specific,
    /// typically during an upward walk of the tree.
    pub fn add(&mut self, entry: AnnotationEntry<T>) {
        self.entries.push(entry);
    }

    /// The [`AnnotationEntry`] objects recorded.
    ///
    /// The first entry is the most specific, typically the one at the leaf of the tree.
    pub fn entries(&self) -> &[AnnotationEntry<T>] {
        &self.entries
    }

    /// The annotations recorded.
    ///
    /// The first entry is the most specific, typically the one at the leaf of the tree. It is
    /// similar to [`entries`](Self::entries) but does not contain other information.
    pub fn annotations(&self) -> impl Iterator<Item = &Rc<T>> {
        self.entries.iter().map(|entry| &entry.annotation)
    }
}

impl<T> Default for AnnotationResult<T> {
    fn default() -> AnnotationResult<T> {
        AnnotationResult::new()
    }
}

/// A layer which annotates its children with a value.
///
/// An annotation is an optional object of any type that, when attached with a layer, can be
/// retrieved using [`BoundaryLayer::find`] or [`BoundaryLayer::find_all_annotations`] with a
/// position. The search process is done recursively, controlled by a concept of being opaque to
/// a type of annotation.
///
/// When an annotation search arrives, this layer defers the same search to each of its
/// children, respecting their opacity. Then it adds this layer's annotation if all of the
/// following restrictions are met:
///
/// * The target type must be identical to the annotated type.
/// * If [`size`](Self::size) is provided, the target position must be contained within the
///   rectangle formed by [`size`](Self::size) and [`offset`](Self::offset).
///
/// This layer is opaque to a type of annotation if any child is also opaque, or if
/// [`opaque`](Self::opaque) is true and the layer's annotation is added.
///
/// Flutter's `AnnotatedRegionLayer`. It is a value carried by the recording rather than a
/// retained layer, because it creates no engine layer; see `PORTING.md`.
pub struct AnnotatedRegionLayer {
    /// The annotated object, which is added to the result if all restrictions are met.
    ///
    /// Dart's `T value`, erased so the recording can carry annotations of every type.
    pub value: Rc<dyn Any>,

    /// The size of the annotated object.
    ///
    /// If [`size`](Self::size) is provided, then the annotation is found only if the target
    /// position is contained by the rectangle formed by [`size`](Self::size) and
    /// [`offset`](Self::offset). Otherwise no such restriction is applied, and clipping can
    /// only be done by the ancestor layers.
    pub size: Option<Size>,

    /// The position of the annotated object.
    ///
    /// The [`offset`](Self::offset) defaults to [`Offset::ZERO`] if not provided, and is
    /// ignored if [`size`](Self::size) is not set.
    ///
    /// The [`offset`](Self::offset) only offsets the clipping rectangle, and does not affect
    /// how the painting or annotation search is propagated to its children.
    pub offset: Offset,

    /// Whether the annotation of this layer should be opaque during an annotation search of its
    /// own type, preventing siblings visually behind it from being searched.
    ///
    /// If [`opaque`](Self::opaque) is true, and this layer does add its annotation
    /// [`value`](Self::value), then the layer will always be opaque during the search.
    ///
    /// If [`opaque`](Self::opaque) is false, or if this layer does not add its annotation, then
    /// the opacity of this layer will be the one returned by the children, meaning that it will
    /// be opaque if any child is opaque.
    ///
    /// Defaults to false. It is effectively useless during [`BoundaryLayer::find`], since the
    /// search process then skips the remaining tree after finding the first annotation.
    pub opaque: bool,
}

impl AnnotatedRegionLayer {
    /// Creates a new layer that annotates its children with `value`; Dart's optional named
    /// arguments are the setters.
    pub fn new(value: Rc<dyn Any>) -> AnnotatedRegionLayer {
        AnnotatedRegionLayer {
            value,
            size: None,
            offset: Offset::ZERO,
            opaque: false,
        }
    }

    /// Dart `AnnotatedRegionLayer(size:)`.
    pub fn size(mut self, size: Size) -> AnnotatedRegionLayer {
        self.size = Some(size);
        self
    }

    /// Dart `AnnotatedRegionLayer(offset:)`.
    pub fn offset(mut self, offset: Offset) -> AnnotatedRegionLayer {
        self.offset = offset;
        self
    }

    /// Dart `AnnotatedRegionLayer(opaque:)`.
    pub fn opaque(mut self, opaque: bool) -> AnnotatedRegionLayer {
        self.opaque = opaque;
        self
    }
}

impl Debug for AnnotatedRegionLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnnotatedRegionLayer")
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("opaque", &self.opaque)
            .finish_non_exhaustive()
    }
}

/// The layer a repaint boundary composites its recording through.
///
/// Flutter's `OffsetLayer` and its subclasses `OpacityLayer` and `TransformLayer`.
/// [`crate::RenderObject::update_composited_layer`] returns one;
/// [`AnyRenderObject::mark_needs_composited_layer_update`] replaces it without repainting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompositedLayer {
    /// Offset from parent in the parent's coordinate system. Set by the parent's paint.
    pub offset: Offset,
    /// Which layer this is.
    pub kind: CompositedLayerKind,
}

/// The subclass of a [`CompositedLayer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompositedLayerKind {
    /// `OffsetLayer`.
    Offset,
    /// `OpacityLayer`. `alpha` is 0 to 255; 255 composites as an `OffsetLayer`.
    Opacity {
        /// The amount to multiply into the alpha channel.
        alpha: i32,
    },
    /// `TransformLayer`.
    Transform {
        /// The matrix to apply, after `offset`.
        transform: Matrix4,
    },
}

impl CompositedLayer {
    /// Flutter's `OffsetLayer({offset})`.
    pub const fn offset_layer(offset: Offset) -> CompositedLayer {
        CompositedLayer {
            offset,
            kind: CompositedLayerKind::Offset,
        }
    }

    /// Flutter's `OpacityLayer({alpha, offset})`.
    pub const fn opacity_layer(alpha: i32, offset: Offset) -> CompositedLayer {
        CompositedLayer {
            offset,
            kind: CompositedLayerKind::Opacity { alpha },
        }
    }

    /// Flutter's `TransformLayer({transform, offset})`.
    pub const fn transform_layer(transform: Matrix4, offset: Offset) -> CompositedLayer {
        CompositedLayer {
            offset,
            kind: CompositedLayerKind::Transform { transform },
        }
    }
}

impl Default for CompositedLayer {
    /// Flutter's `OffsetLayer()`.
    fn default() -> CompositedLayer {
        CompositedLayer::offset_layer(Offset::ZERO)
    }
}

/// A key that identifies a group of backdrop filters that share one backdrop.
///
/// Every `BackdropFilterLayer` with the same key blurs the scene once, as of the first of
/// them, which is what Flutter's `BackdropGroup` trades for the cost of one filter pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BackdropKey(u64);

static NEXT_BACKDROP_KEY: AtomicU64 = AtomicU64::new(0);

impl BackdropKey {
    /// Create a new [`BackdropKey`].
    #[expect(
        clippy::new_without_default,
        reason = "each call creates a distinct key; Default would suggest a neutral value"
    )]
    pub fn new() -> BackdropKey {
        BackdropKey(NEXT_BACKDROP_KEY.fetch_add(1, Ordering::Relaxed))
    }
}

/// One retained piece of a repaint boundary's recording: Flutter's layer subclasses as data.
pub(crate) enum PaintItem {
    /// `PictureLayer`. `cache` is Flutter's `isComplexHint`: a raster-cache candidate.
    Picture { picture: Arc<Picture>, cache: bool },
    /// A child repaint boundary, composited through its own [`BoundaryLayer`].
    ChildBoundary(AnyRenderObject),
    /// `OpacityLayer` from `PaintingContext::push_opacity`. Closed by [`Pop`](Self::Pop).
    PushOpacity { alpha: i32, offset: Offset },
    /// `BackdropFilterLayer` from `PaintingContext::push_backdrop_filter`: blurs what is
    /// already under `bounds`, then paints the children on top.
    PushBackdropFilter {
        filter: ImageFilter,
        blend_mode: BlendMode,
        backdrop_key: Option<BackdropKey>,
        bounds: Rect,
        offset: Offset,
    },
    /// `ClipRectLayer`.
    PushClipRect {
        clip_rect: Rect,
        clip_behavior: Clip,
    },
    /// `ClipRRectLayer`. `offset` is applied to the shape at composition.
    PushClipRRect {
        clip_rrect: RRect,
        offset: Offset,
        bounds: Rect,
        clip_behavior: Clip,
    },
    /// `ClipPathLayer`. `offset` is applied to the path at composition.
    PushClipPath {
        clip_path: Arc<Path>,
        offset: Offset,
        bounds: Rect,
        clip_behavior: Clip,
    },
    /// `TransformLayer` from `PaintingContext::push_transform`.
    PushTransform { transform: Matrix4 },
    /// `AnnotatedRegionLayer` from `PaintingContext::push_annotated_region`: it paints nothing
    /// of its own and only answers [`BoundaryLayer::find`].
    PushAnnotatedRegion(AnnotatedRegionLayer),
    /// Closes the innermost `Push*`.
    Pop,
}

/// A repaint boundary's layer: the composited wrapper plus the retained recording.
///
/// Flutter's `RenderObject.layer`, an `OffsetLayer` whose children are the recording.
pub struct BoundaryLayer {
    pub(crate) composited: CompositedLayer,
    pub(crate) items: Vec<PaintItem>,
    /// Flutter's `Layer.attached`: composited by an attached parent. The root layer is attached
    /// by `schedule_initial_paint`.
    pub(crate) attached: bool,
    /// The boundary whose recording composites this one. Flutter's `Layer.parent`.
    pub(crate) parent: Option<AnyRenderObject>,
}

impl BoundaryLayer {
    pub(crate) fn new(composited: CompositedLayer, attached: bool) -> BoundaryLayer {
        BoundaryLayer {
            composited,
            items: Vec::new(),
            attached,
            parent: None,
        }
    }

    /// The composited wrapper.
    pub fn composited(&self) -> CompositedLayer {
        self.composited
    }

    /// Flutter's `Layer.attached`.
    pub fn attached(&self) -> bool {
        self.attached
    }

    /// Composes the retained recording into `canvas`.
    ///
    /// Flutter's `ContainerLayer.buildScene` / `addToScene`. `RenderView.compositeFrame` calls
    /// it on the root boundary's layer.
    pub fn add_to_scene(&self, app: &App, canvas: &mut Canvas) {
        // Flutter's `OpacityLayer.addToScene`: a layer without children adds nothing.
        if self.items.is_empty() {
            return;
        }
        push_composited(canvas, &self.composited);
        for item in &self.items {
            match item {
                PaintItem::Picture { picture, cache } => {
                    if *cache {
                        canvas.draw_display_list_cached(picture);
                    } else {
                        canvas.draw_display_list(picture);
                    }
                }
                PaintItem::ChildBoundary(child) => {
                    if let Some(layer) = child.layer(app) {
                        layer.add_to_scene(app, canvas);
                    }
                }
                PaintItem::PushOpacity { alpha, offset } => push_opacity(canvas, *alpha, *offset),
                PaintItem::PushBackdropFilter {
                    filter,
                    blend_mode,
                    backdrop_key,
                    bounds,
                    offset,
                } => push_backdrop_filter(
                    canvas,
                    filter,
                    *blend_mode,
                    *backdrop_key,
                    *bounds,
                    *offset,
                ),
                PaintItem::PushClipRect {
                    clip_rect,
                    clip_behavior,
                } => push_clip(canvas, *clip_behavior, *clip_rect, |canvas| {
                    canvas.clip_rect(*clip_rect, ClipOp::Intersect);
                }),
                PaintItem::PushClipRRect {
                    clip_rrect,
                    offset,
                    bounds,
                    clip_behavior,
                } => push_clip(canvas, *clip_behavior, *bounds, |canvas| {
                    translate(canvas, *offset);
                    canvas.clip_rrect_radii_elliptical(
                        clip_rrect.outer_rect(),
                        rrect_radii_elliptical(*clip_rrect),
                        ClipOp::Intersect,
                    );
                    translate(canvas, -*offset);
                }),
                PaintItem::PushClipPath {
                    clip_path,
                    offset,
                    bounds,
                    clip_behavior,
                } => push_clip(canvas, *clip_behavior, *bounds, |canvas| {
                    translate(canvas, *offset);
                    canvas.clip_path(clip_path, FillRule::NonZero, ClipOp::Intersect);
                    translate(canvas, -*offset);
                }),
                PaintItem::PushTransform { transform } => {
                    canvas.save();
                    canvas.concat(transform);
                }
                // Dart's `AnnotatedRegionLayer.addToScene` is `ContainerLayer`'s: it adds its
                // children and no scope of its own. The `Pop` still has to find one.
                PaintItem::PushAnnotatedRegion(_) => canvas.save(),
                PaintItem::Pop => canvas.restore(),
            }
        }
        canvas.restore();
    }
}

impl BoundaryLayer {
    /// Search this layer and its subtree for the first annotation of type `T` under the point
    /// described by `local_position`.
    ///
    /// Returns `None` if no matching annotation is found.
    ///
    /// An annotation is an optional object of any type that can be carried with a layer. An
    /// annotation can be found at a location as long as the owner layer contains the location
    /// and is walked to. The annotations are searched by first visiting each child recursively,
    /// then this layer, resulting in an order from visually front to back.
    ///
    /// The common way for a value to be found here is by pushing an [`AnnotatedRegionLayer`]
    /// into the recording, which `RenderAnnotatedRegion` does.
    pub fn find<T: Any>(&self, app: &App, local_position: Offset) -> Option<Rc<T>> {
        let mut result = AnnotationResult::new();
        self.find_annotations(app, &mut result, local_position, true);
        result
            .entries()
            .first()
            .map(|entry| Rc::clone(&entry.annotation))
    }

    /// Search this layer and its subtree for all annotations of type `T` under the point
    /// described by `local_position`.
    ///
    /// Returns a result with empty entries if no matching annotations are found. The first
    /// entry is the innermost annotated region that contains the point.
    pub fn find_all_annotations<T: Any>(
        &self,
        app: &App,
        local_position: Offset,
    ) -> AnnotationResult<T> {
        let mut result = AnnotationResult::new();
        self.find_annotations(app, &mut result, local_position, false);
        result
    }

    /// Flutter's `OffsetLayer.findAnnotations` (and, for a transform, `TransformLayer`'s) over
    /// the composited wrapper, then `ContainerLayer.findAnnotations` over the recording.
    ///
    /// The return value is the opacity of this layer and its subtree at this position: when it
    /// is true, the caller must skip whatever this layer paints over.
    fn find_annotations<T: Any>(
        &self,
        app: &App,
        result: &mut AnnotationResult<T>,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let position = match self.composited.kind {
            CompositedLayerKind::Offset | CompositedLayerKind::Opacity { .. } => local_position,
            CompositedLayerKind::Transform { transform } => {
                let Some(inverted) = transform.invert() else {
                    return false;
                };
                reveal_painting::transform_point(&inverted, local_position)
            }
        };
        find_annotations_in(
            app,
            &self.items,
            result,
            position - self.composited.offset,
            only_first,
        )
    }
}

/// Flutter's `ContainerLayer.findAnnotations`: each recorded item from last to first, stopping
/// at the first one that absorbs the search.
fn find_annotations_in<T: Any>(
    app: &App,
    items: &[PaintItem],
    result: &mut AnnotationResult<T>,
    local_position: Offset,
    only_first: bool,
) -> bool {
    for scope in scopes(items).into_iter().rev() {
        let is_absorbed = find_annotations_in_item(
            app,
            &items[scope.head],
            &items[scope.children],
            result,
            local_position,
            only_first,
        );
        if is_absorbed {
            return true;
        }
        if only_first && !result.entries().is_empty() {
            return is_absorbed;
        }
    }
    false
}

/// One recorded item's `findAnnotations`, dispatched on which layer the item stands for.
fn find_annotations_in_item<T: Any>(
    app: &App,
    item: &PaintItem,
    children: &[PaintItem],
    result: &mut AnnotationResult<T>,
    local_position: Offset,
    only_first: bool,
) -> bool {
    match item {
        // `PictureLayer.findAnnotations` is `Layer`'s: no annotations, no absorption.
        PaintItem::Picture { .. } => false,
        PaintItem::ChildBoundary(child) => child
            .layer(app)
            .is_some_and(|layer| layer.find_annotations(app, result, local_position, only_first)),
        // `OpacityLayer` and `BackdropFilterLayer` reach their children through the offset the
        // recording translates by.
        PaintItem::PushOpacity { offset, .. } | PaintItem::PushBackdropFilter { offset, .. } => {
            find_annotations_in(app, children, result, local_position - *offset, only_first)
        }
        PaintItem::PushTransform { transform } => {
            let Some(inverted) = transform.invert() else {
                return false;
            };
            let transformed = reveal_painting::transform_point(&inverted, local_position);
            find_annotations_in(app, children, result, transformed, only_first)
        }
        PaintItem::PushClipRect { clip_rect, .. } => {
            clip_rect.contains(local_position)
                && find_annotations_in(app, children, result, local_position, only_first)
        }
        PaintItem::PushClipRRect {
            clip_rrect, offset, ..
        } => {
            clip_rrect.shift(*offset).contains(local_position)
                && find_annotations_in(app, children, result, local_position, only_first)
        }
        PaintItem::PushClipPath {
            clip_path, offset, ..
        } => {
            clip_path.contains((local_position - *offset).into(), FillRule::NonZero)
                && find_annotations_in(app, children, result, local_position, only_first)
        }
        PaintItem::PushAnnotatedRegion(region) => {
            let mut is_absorbed =
                find_annotations_in(app, children, result, local_position, only_first);
            if !result.entries().is_empty() && only_first {
                return is_absorbed;
            }
            if let Some(size) = region.size
                && !(region.offset & size).contains(local_position)
            {
                return is_absorbed;
            }
            if let Ok(annotation) = Rc::clone(&region.value).downcast::<T>() {
                is_absorbed = is_absorbed || region.opaque;
                result.add(AnnotationEntry::new(
                    annotation,
                    local_position - region.offset,
                ));
            }
            is_absorbed
        }
        PaintItem::Pop => unreachable!("a `Pop` is consumed by the scope it closes"),
    }
}

/// One top-level recorded item: the index of the item itself, and the range its nested items
/// span (empty for an item that opens no scope).
struct Scope {
    head: usize,
    children: std::ops::Range<usize>,
}

/// Splits a recording into its top-level items, in paint order.
///
/// A `Push*` item owns everything up to its matching [`PaintItem::Pop`]; the recording is
/// well-formed because only `PaintingContext::push_layer` writes the pair.
fn scopes(items: &[PaintItem]) -> Vec<Scope> {
    let mut scopes = Vec::new();
    let mut index = 0;
    while index < items.len() {
        if !items[index].opens_scope() {
            scopes.push(Scope {
                head: index,
                children: index..index,
            });
            index += 1;
            continue;
        }
        let mut depth = 1usize;
        let mut end = index + 1;
        while end < items.len() {
            match &items[end] {
                PaintItem::Pop => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                item if item.opens_scope() => depth += 1,
                _ => {}
            }
            end += 1;
        }
        debug_assert!(end < items.len(), "every pushed scope is closed by a `Pop`");
        scopes.push(Scope {
            head: index,
            children: index + 1..end.min(items.len()),
        });
        index = end + 1;
    }
    scopes
}

impl PaintItem {
    /// Whether this item opens a scope that a [`PaintItem::Pop`] closes.
    fn opens_scope(&self) -> bool {
        matches!(
            self,
            PaintItem::PushOpacity { .. }
                | PaintItem::PushBackdropFilter { .. }
                | PaintItem::PushClipRect { .. }
                | PaintItem::PushClipRRect { .. }
                | PaintItem::PushClipPath { .. }
                | PaintItem::PushTransform { .. }
                | PaintItem::PushAnnotatedRegion(_)
        )
    }
}

fn translate(canvas: &mut Canvas, offset: Offset) {
    canvas.translate(offset.dx() as f32, offset.dy() as f32);
}

/// Flutter's `OffsetLayer.addToScene` / `OpacityLayer.addToScene` / `TransformLayer.addToScene`
/// up to `addChildrenToScene`. One scope; `add_to_scene` restores it.
fn push_composited(canvas: &mut Canvas, layer: &CompositedLayer) {
    match layer.kind {
        CompositedLayerKind::Offset => {
            canvas.save();
            translate(canvas, layer.offset);
        }
        CompositedLayerKind::Opacity { alpha } => push_opacity(canvas, alpha, layer.offset),
        CompositedLayerKind::Transform { transform } => {
            canvas.save();
            translate(canvas, layer.offset);
            canvas.concat(&transform);
        }
    }
}

/// `SceneBuilder.pushBackdropFilter`: valo's `saveLayer(bounds, paint, backdrop)` opens a
/// layer pre-filled with a blur of everything beneath it; the children paint over that glass.
fn push_backdrop_filter(
    canvas: &mut Canvas,
    filter: &ImageFilter,
    blend_mode: BlendMode,
    backdrop_key: Option<BackdropKey>,
    bounds: Rect,
    offset: Offset,
) {
    let (sigma_x, filter_bounds) = backdrop_blur(filter).unwrap_or((0.0, None));
    let mut backdrop = Backdrop::blur(sigma_x as f32);
    if let Some(key) = backdrop_key {
        backdrop = backdrop.shared(key.0);
    }
    let paint = Paint {
        blend_mode,
        ..Paint::default()
    };
    canvas.save_layer_backdrop(
        Some(filter_bounds.unwrap_or(bounds).into()),
        &paint,
        backdrop,
    );
    translate(canvas, offset);
}

/// The blur a valo [`Backdrop`] can sample the scene with, out of an arbitrary filter tree:
/// its sigma and the bounds it is confined to.
///
/// A colour filter contributes nothing — valo's backdrop is a blur, and moving the colour
/// filter onto the layer paint would filter the children too. See the host's `PORTING.md`.
fn backdrop_blur(filter: &ImageFilter) -> Option<(f64, Option<Rect>)> {
    match filter {
        ImageFilter::Blur {
            sigma_x, bounds, ..
        } => Some((*sigma_x, *bounds)),
        ImageFilter::Color(_) => None,
        ImageFilter::Compose { outer, inner } => {
            backdrop_blur(inner).or_else(|| backdrop_blur(outer))
        }
    }
}

/// `SceneBuilder.pushOpacity`, or `pushOffset` when fully opaque or opacity layers are disabled,
/// as Flutter's `OpacityLayer.addToScene` chooses.
fn push_opacity(canvas: &mut Canvas, alpha: i32, offset: Offset) {
    if alpha < 255 && !debug_disable_opacity_layers() {
        let paint = Paint {
            color: Color::from_argb(alpha, 0, 0, 0).into(),
            ..Paint::default()
        };
        canvas.save_layer(None, &paint);
    } else {
        canvas.save();
    }
    translate(canvas, offset);
}

/// One clip scope. `Clip::AntiAliasWithSaveLayer` opens a layer over `bounds`, as
/// `ClipContext.clipPathAndPaint` does.
fn push_clip(
    canvas: &mut Canvas,
    clip_behavior: Clip,
    bounds: Rect,
    clip: impl FnOnce(&mut Canvas),
) {
    match clip_behavior {
        Clip::None | Clip::HardEdge | Clip::AntiAlias => canvas.save(),
        Clip::AntiAliasWithSaveLayer => canvas.save_layer(Some(bounds.into()), &Paint::default()),
    }
    clip(canvas);
}

impl AnyRenderObject {
    /// Flutter's `Layer.attach` on this boundary's layer, then on the boundaries it composites.
    pub(crate) fn attach_layer(self, app: &mut App) {
        let Some(layer) = self.layer_mut(app) else {
            return;
        };
        if layer.attached {
            return;
        }
        layer.attached = true;
        for child in self.layer_children(app) {
            child.attach_layer(app);
        }
    }

    /// Flutter's `Layer.detach` on this boundary's layer, then on the boundaries it composites.
    pub(crate) fn detach_layer(self, app: &mut App) {
        let Some(layer) = self.layer_mut(app) else {
            return;
        };
        if !layer.attached {
            return;
        }
        layer.attached = false;
        for child in self.layer_children(app) {
            child.detach_layer(app);
        }
    }

    /// Flutter's `Layer.remove`: unlinks this boundary's layer from the recording that
    /// composites it.
    pub(crate) fn remove_layer(self, app: &mut App) {
        let Some(parent) = self.layer(app).and_then(|layer| layer.parent) else {
            return;
        };
        if let Some(parent_layer) = parent.layer_mut(app) {
            parent_layer
                .items
                .retain(|item| !matches!(item, PaintItem::ChildBoundary(child) if *child == self));
        }
        self.drop_layer_from_parent(app);
    }

    /// Flutter's `ContainerLayer.removeAllChildren` on this boundary's layer.
    pub(crate) fn remove_all_layer_children(self, app: &mut App) {
        let children = self.layer_children(app);
        if let Some(layer) = self.layer_mut(app) {
            layer.items.clear();
        }
        for child in children {
            child.drop_layer_from_parent(app);
        }
    }

    /// Flutter's `ContainerLayer._dropChild` from the child's side.
    pub(crate) fn drop_layer_from_parent(self, app: &mut App) {
        let Some(layer) = self.layer_mut(app) else {
            return;
        };
        layer.parent = None;
        if layer.attached {
            self.detach_layer(app);
        }
    }

    /// The boundaries this boundary's recording composites.
    pub(crate) fn layer_children(self, app: &App) -> Vec<AnyRenderObject> {
        self.layer(app)
            .map(|layer| {
                layer
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        PaintItem::ChildBoundary(child) => Some(*child),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::valo::Op;

    use super::*;

    fn picture() -> PaintItem {
        let mut canvas = Canvas::new();
        canvas.draw_rect(
            Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            &Paint {
                color: Color::from_argb(255, 0, 0, 0).into(),
                ..Paint::default()
            },
        );
        PaintItem::Picture {
            picture: canvas.build().into(),
            cache: false,
        }
    }

    fn compose(layer: &BoundaryLayer) -> Vec<Op> {
        let app = App::new();
        let mut canvas = Canvas::new();
        layer.add_to_scene(&app, &mut canvas);
        canvas.build().ops().to_vec()
    }

    /// `layers_test.dart`: `OpacityLayer does not push an OffsetLayer if there are no children`.
    #[test]
    fn opacity_layer_without_children_adds_nothing() {
        let layer = BoundaryLayer::new(CompositedLayer::opacity_layer(128, Offset::ZERO), true);
        assert!(compose(&layer).is_empty());
    }

    #[test]
    fn opacity_layer_pushes_a_layer_below_255_and_an_offset_at_255() {
        let mut layer = BoundaryLayer::new(CompositedLayer::opacity_layer(128, Offset::ZERO), true);
        layer.items.push(picture());
        let ops = compose(&layer);
        assert!(matches!(ops[0], Op::SaveLayer { .. }), "{ops:?}");
        assert!(
            ops.iter()
                .any(|op| matches!(op, Op::DrawDisplayList { .. }))
        );
        assert!(matches!(ops.last(), Some(Op::Restore)));

        layer.composited = CompositedLayer::opacity_layer(255, Offset::ZERO);
        let ops = compose(&layer);
        assert!(matches!(ops[0], Op::Save), "{ops:?}");
    }

    /// The recording a `PushAnnotatedRegion` opens and a `Pop` closes.
    fn region(layer: AnnotatedRegionLayer) -> Vec<PaintItem> {
        vec![PaintItem::PushAnnotatedRegion(layer), PaintItem::Pop]
    }

    #[test]
    fn a_clip_gates_the_annotations_recorded_under_it() {
        let app = App::new();
        let mut layer = BoundaryLayer::new(CompositedLayer::default(), true);
        layer.items.push(PaintItem::PushClipRect {
            clip_rect: Rect::from_ltwh(0.0, 0.0, 50.0, 50.0),
            clip_behavior: Clip::HardEdge,
        });
        layer
            .items
            .extend(region(AnnotatedRegionLayer::new(Rc::new(7u32))));
        layer.items.push(PaintItem::Pop);

        assert_eq!(
            layer.find::<u32>(&app, Offset::new(10.0, 10.0)).as_deref(),
            Some(&7)
        );
        assert!(
            layer.find::<u32>(&app, Offset::new(60.0, 10.0)).is_none(),
            "outside the clip"
        );
    }

    #[test]
    fn a_transform_maps_the_search_point_into_what_it_records() {
        let app = App::new();
        let mut layer = BoundaryLayer::new(CompositedLayer::default(), true);
        layer.items.push(PaintItem::PushTransform {
            transform: Matrix4::translation(10.0, 0.0),
        });
        layer.items.extend(region(
            AnnotatedRegionLayer::new(Rc::new(7u32)).size(Size::new(20.0, 20.0)),
        ));
        layer.items.push(PaintItem::Pop);

        assert!(layer.find::<u32>(&app, Offset::new(15.0, 5.0)).is_some());
        assert!(
            layer.find::<u32>(&app, Offset::new(5.0, 5.0)).is_none(),
            "left of the translated region"
        );
    }

    #[test]
    fn an_opaque_region_absorbs_the_search_for_what_is_recorded_behind_it() {
        let app = App::new();
        let behind = AnnotatedRegionLayer::new(Rc::new(1u32));
        let front = AnnotatedRegionLayer::new(Rc::new(2u32)).opaque(true);
        let mut layer = BoundaryLayer::new(CompositedLayer::default(), true);
        layer.items.extend(region(behind));
        layer.items.extend(region(front));

        let result = layer.find_all_annotations::<u32>(&app, Offset::ZERO);
        assert_eq!(
            result
                .annotations()
                .map(|value| **value)
                .collect::<Vec<_>>(),
            vec![2],
            "an opaque region hides its siblings"
        );

        let mut layer = BoundaryLayer::new(CompositedLayer::default(), true);
        layer
            .items
            .extend(region(AnnotatedRegionLayer::new(Rc::new(1u32))));
        layer
            .items
            .extend(region(AnnotatedRegionLayer::new(Rc::new(2u32))));
        let result = layer.find_all_annotations::<u32>(&app, Offset::ZERO);
        assert_eq!(
            result
                .annotations()
                .map(|value| **value)
                .collect::<Vec<_>>(),
            vec![2, 1],
            "front to back when neither is opaque"
        );
    }

    #[test]
    fn pushed_items_open_and_close_scopes() {
        let mut layer = BoundaryLayer::new(CompositedLayer::default(), true);
        layer.items.push(PaintItem::PushClipRect {
            clip_rect: Rect::from_ltwh(0.0, 0.0, 5.0, 5.0),
            clip_behavior: Clip::HardEdge,
        });
        layer.items.push(picture());
        layer.items.push(PaintItem::Pop);
        let ops = compose(&layer);
        let saves = ops.iter().filter(|op| matches!(op, Op::Save)).count();
        let restores = ops.iter().filter(|op| matches!(op, Op::Restore)).count();
        assert_eq!((saves, restores), (2, 2), "{ops:?}");
    }
}
