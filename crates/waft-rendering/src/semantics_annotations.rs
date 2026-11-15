//! Flutter counterpart: `rendering/object.dart` (`SemanticsAnnotationsMixin`).
//!
//! The mixin keeps what the semantics widgets give it. What reads it waits with the semantics
//! tree: `describeSemanticsConfiguration`, which copies these values and registers the
//! `_perform*` handlers, the `visitChildrenForSemantics` override that `excludeSemantics`
//! controls, and the `markNeedsSemanticsUpdate` each setter ends with. The setters keep Dart's
//! comparisons and early returns, so they change exactly when Dart's would mark the
//! semantics dirty.

use std::rc::Rc;

use waft_embedder::{Locale, TextDirection};
use waft_foundation::App;

use crate::object::{RenderHandle, RenderObject};
use crate::semantics::{AttributedString, SemanticsProperties};

/// Flutter's `SemanticsAnnotationsMixin` fields.
#[derive(Debug, Default)]
pub struct SemanticsAnnotationsData {
    properties: Option<Rc<SemanticsProperties>>,
    container: bool,
    explicit_child_nodes: bool,
    exclude_semantics: bool,
    block_user_actions: bool,
    locale_for_subtree: Option<Locale>,
    attributed_label: Option<AttributedString>,
    attributed_value: Option<AttributedString>,
    attributed_increased_value: Option<AttributedString>,
    attributed_decreased_value: Option<AttributedString>,
    attributed_hint: Option<AttributedString>,
    text_direction: Option<TextDirection>,
}

impl SemanticsAnnotationsData {
    /// Nothing set yet: the host calls
    /// [`init_semantics_annotations`](SemanticsAnnotationsMixin::init_semantics_annotations)
    /// in its constructor.
    pub fn new() -> SemanticsAnnotationsData {
        SemanticsAnnotationsData::default()
    }
}

/// A mixin for [`RenderObject`]s that want to annotate the `SemanticsNode`
/// for their subtree.
pub trait SemanticsAnnotationsMixin: RenderObject {
    /// Mixin field access.
    fn semantics_annotations_data(self: RenderHandle<Self>, app: &App)
    -> &SemanticsAnnotationsData;

    /// See [`semantics_annotations_data`](Self::semantics_annotations_data).
    fn semantics_annotations_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut SemanticsAnnotationsData;

    /// Initializes the semantics annotations for this mixin.
    #[allow(clippy::too_many_arguments)]
    fn init_semantics_annotations(
        self: RenderHandle<Self>,
        app: &mut App,
        properties: Rc<SemanticsProperties>,
        container: bool,
        explicit_child_nodes: bool,
        exclude_semantics: bool,
        block_user_actions: bool,
        locale_for_subtree: Option<Locale>,
        text_direction: Option<TextDirection>,
    ) {
        let data = self.semantics_annotations_data_mut(app);
        data.properties = Some(properties.clone());
        data.container = container;
        data.explicit_child_nodes = explicit_child_nodes;
        data.exclude_semantics = exclude_semantics;
        data.block_user_actions = block_user_actions;
        data.locale_for_subtree = locale_for_subtree;
        data.text_direction = text_direction;
        self.update_attributed_fields(app, &properties);
    }

    /// All of the [`SemanticsProperties`] for this [`SemanticsAnnotationsMixin`].
    fn properties(self: RenderHandle<Self>, app: &App) -> Rc<SemanticsProperties> {
        self.semantics_annotations_data(app)
            .properties
            .clone()
            .expect("set by init_semantics_annotations")
    }

    /// Sets [`properties`](Self::properties).
    fn set_properties(self: RenderHandle<Self>, app: &mut App, value: Rc<SemanticsProperties>) {
        if Rc::ptr_eq(&self.properties(app), &value) {
            return;
        }
        self.semantics_annotations_data_mut(app).properties = Some(value.clone());
        self.update_attributed_fields(app, &value);
    }

    /// If 'container' is true, this [`RenderObject`] will introduce a new
    /// node in the semantics tree. Otherwise, the semantics will be
    /// merged with the semantics of any ancestors.
    ///
    /// Whether descendants of this [`RenderObject`] can add their semantic information
    /// to the `SemanticsNode` introduced by this configuration is controlled by
    /// [`explicit_child_nodes`](Self::explicit_child_nodes).
    fn container(self: RenderHandle<Self>, app: &App) -> bool {
        self.semantics_annotations_data(app).container
    }

    /// Sets [`container`](Self::container).
    fn set_container(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.container(app) == value {
            return;
        }
        self.semantics_annotations_data_mut(app).container = value;
    }

    /// Whether descendants of this [`RenderObject`] are allowed to add semantic
    /// information to the `SemanticsNode` annotated by this widget.
    ///
    /// When set to false descendants are allowed to annotate `SemanticsNode`s of
    /// their parent with the semantic information they want to contribute to the
    /// semantic tree.
    /// When set to true the only way for descendants to contribute semantic
    /// information to the semantic tree is to introduce new explicit
    /// `SemanticsNode`s to the tree.
    ///
    /// This setting is often used in combination with
    /// `SemanticsConfiguration.isSemanticBoundary` to create semantic boundaries
    /// that are either writable or not for children.
    fn explicit_child_nodes(self: RenderHandle<Self>, app: &App) -> bool {
        self.semantics_annotations_data(app).explicit_child_nodes
    }

    /// Sets [`explicit_child_nodes`](Self::explicit_child_nodes).
    fn set_explicit_child_nodes(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.semantics_annotations_data(app).explicit_child_nodes == value {
            return;
        }
        self.semantics_annotations_data_mut(app)
            .explicit_child_nodes = value;
    }

    /// Whether descendants of this [`RenderObject`] should have their semantic
    /// information ignored.
    ///
    /// When this flag is set to true, all child semantics nodes are ignored.
    /// This can be used as a convenience for cases where a child is wrapped in
    /// an `ExcludeSemantics` widget and then another `Semantics` widget.
    fn exclude_semantics(self: RenderHandle<Self>, app: &App) -> bool {
        self.semantics_annotations_data(app).exclude_semantics
    }

    /// Sets [`exclude_semantics`](Self::exclude_semantics).
    fn set_exclude_semantics(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.semantics_annotations_data(app).exclude_semantics == value {
            return;
        }
        self.semantics_annotations_data_mut(app).exclude_semantics = value;
    }

    /// Whether to block user interactions for the semantics subtree.
    ///
    /// Setting this true prevents user from activating pointer related
    /// `SemanticsAction`s, such as `SemanticsAction::TAP` or
    /// `SemanticsAction::LONG_PRESS`.
    fn block_user_actions(self: RenderHandle<Self>, app: &App) -> bool {
        self.semantics_annotations_data(app).block_user_actions
    }

    /// Sets [`block_user_actions`](Self::block_user_actions).
    fn set_block_user_actions(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.semantics_annotations_data(app).block_user_actions == value {
            return;
        }
        self.semantics_annotations_data_mut(app).block_user_actions = value;
    }

    /// The [`Locale`] for the semantics subtree.
    ///
    /// Setting this to null will inherit locale from ancestor semantics node.
    fn locale_for_subtree(self: RenderHandle<Self>, app: &App) -> Option<Locale> {
        self.semantics_annotations_data(app)
            .locale_for_subtree
            .clone()
    }

    /// Sets [`locale_for_subtree`](Self::locale_for_subtree).
    fn set_locale_for_subtree(self: RenderHandle<Self>, app: &mut App, value: Option<Locale>) {
        if self.semantics_annotations_data(app).locale_for_subtree == value {
            return;
        }
        self.semantics_annotations_data_mut(app).locale_for_subtree = value;
    }

    /// Flutter's `_updateAttributedFields`.
    fn update_attributed_fields(
        self: RenderHandle<Self>,
        app: &mut App,
        value: &SemanticsProperties,
    ) {
        let attributed_label = effective_attributed_label(value);
        let attributed_value = effective_attributed_value(value);
        let attributed_increased_value = effective_attributed_increased_value(value);
        let attributed_decreased_value =
            effective_attributed_decreased_value(&self.properties(app), value);
        let attributed_hint = effective_attributed_hint(value);
        let data = self.semantics_annotations_data_mut(app);
        data.attributed_label = attributed_label;
        data.attributed_value = attributed_value;
        data.attributed_increased_value = attributed_increased_value;
        data.attributed_decreased_value = attributed_decreased_value;
        data.attributed_hint = attributed_hint;
    }

    /// Dart's `_attributedLabel`: the label `describeSemanticsConfiguration` will report.
    fn attributed_label(self: RenderHandle<Self>, app: &App) -> Option<AttributedString> {
        self.semantics_annotations_data(app)
            .attributed_label
            .clone()
    }

    /// Dart's `_attributedValue`.
    fn attributed_value(self: RenderHandle<Self>, app: &App) -> Option<AttributedString> {
        self.semantics_annotations_data(app)
            .attributed_value
            .clone()
    }

    /// Dart's `_attributedIncreasedValue`.
    fn attributed_increased_value(self: RenderHandle<Self>, app: &App) -> Option<AttributedString> {
        self.semantics_annotations_data(app)
            .attributed_increased_value
            .clone()
    }

    /// Dart's `_attributedDecreasedValue`.
    fn attributed_decreased_value(self: RenderHandle<Self>, app: &App) -> Option<AttributedString> {
        self.semantics_annotations_data(app)
            .attributed_decreased_value
            .clone()
    }

    /// Dart's `_attributedHint`.
    fn attributed_hint(self: RenderHandle<Self>, app: &App) -> Option<AttributedString> {
        self.semantics_annotations_data(app).attributed_hint.clone()
    }

    /// If non-null, sets the `SemanticsNode.textDirection` semantic to the given
    /// value.
    ///
    /// This must not be null if [`SemanticsProperties::attributed_label`],
    /// [`SemanticsProperties::attributed_hint`],
    /// [`SemanticsProperties::attributed_value`],
    /// [`SemanticsProperties::attributed_increased_value`], or
    /// [`SemanticsProperties::attributed_decreased_value`] are not null.
    fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.semantics_annotations_data(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: Option<TextDirection>) {
        if self.text_direction(app) == value {
            return;
        }
        self.semantics_annotations_data_mut(app).text_direction = value;
    }
}

fn effective_attributed_label(value: &SemanticsProperties) -> Option<AttributedString> {
    value
        .attributed_label
        .clone()
        .or_else(|| value.label.clone().map(AttributedString::new))
}

fn effective_attributed_value(value: &SemanticsProperties) -> Option<AttributedString> {
    value
        .attributed_value
        .clone()
        .or_else(|| value.value.clone().map(AttributedString::new))
}

fn effective_attributed_increased_value(value: &SemanticsProperties) -> Option<AttributedString> {
    value
        .attributed_increased_value
        .clone()
        .or_else(|| value.increased_value.clone().map(AttributedString::new))
}

/// Dart reads the attributed value off the mixin's `properties` and the plain one off
/// `value`; both are the same object wherever it is called.
fn effective_attributed_decreased_value(
    properties: &SemanticsProperties,
    value: &SemanticsProperties,
) -> Option<AttributedString> {
    properties
        .attributed_decreased_value
        .clone()
        .or_else(|| value.decreased_value.clone().map(AttributedString::new))
}

fn effective_attributed_hint(value: &SemanticsProperties) -> Option<AttributedString> {
    value
        .attributed_hint
        .clone()
        .or_else(|| value.hint.clone().map(AttributedString::new))
}
