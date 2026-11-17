//! Flutter counterpart: material_ui's `card.dart`.

use waft_embedder::{Clip, Color};
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder, ShapeBorderRef,
};
use waft_widgets::{
    BuildContext, IntoWidget, KeyRef, Padding, Semantics, StatelessWidget, WidgetRef,
};

use crate::card_theme::CardThemeDataGetters;
use crate::card_theme::{CardTheme, CardThemeData};
use crate::generated::card_defaults_m3::CardDefaultsM3;
use crate::generated::filled_card_defaults_m3::FilledCardDefaultsM3;
use crate::generated::outlined_card_defaults_m3::OutlinedCardDefaultsM3;
use crate::material::{Material, MaterialType};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardVariant {
    Elevated,
    Filled,
    Outlined,
}

/// A Material Design card: a panel with slightly rounded corners and an
/// elevation shadow.
///
/// A card is a sheet of [`Material`] used to represent some related information,
/// for example an album, a geographical location, a meal, contact details, etc.
///
/// Sometimes the primary action area of a card is the card itself. Cards can be
/// one large touch target that shows a detail screen when tapped.
///
/// For Material Design 2 (when `ThemeData::use_material3` is false), there is a
/// single card type: the elevated card. In that mode the named constructors
/// ([`Card::filled`], [`Card::outlined`]) behave the same as the default [`Card::new`].
///
/// For Material Design 3 (when `ThemeData::use_material3` is true), three visual
/// variants are available: the default [`Card::new`] (elevated), [`Card::filled`], and
/// [`Card::outlined`]. All variants share the same theme class, [`CardThemeData`],
/// so theme properties (for example [`CardThemeData::shape`]) apply to every card
/// variant within the theme's scope.
///
/// See also:
///
///  * `ListTile`, to display icons and text in a card.
///  * `ShowDialog`, to display a modal card.
///  * <https://material.io/design/components/cards.html>
///  * <https://m3.material.io/components/cards>
#[derive(Debug)]
pub struct Card {
    pub key: Option<KeyRef>,

    /// The card's background color.
    ///
    /// Defines the card's [`Material::color`].
    ///
    /// If this property is null then the ambient [`CardTheme::color`] is used. If that is null,
    /// and `ThemeData::use_material3` is true, then `ColorScheme::surface_container_low` of
    /// `ThemeData::color_scheme` is used. Otherwise, `ThemeData::card_color` is used.
    pub color: Option<Color>,

    /// The color to paint the shadow below the card.
    ///
    /// If null then the ambient [`CardThemeData::shadow_color`] is used.
    /// If that's null too, then the overall theme's `ThemeData::shadow_color`
    /// (default black) is used.
    pub shadow_color: Option<Color>,

    /// The color used as an overlay on [`color`](Self::color) to indicate elevation.
    ///
    /// This is not recommended for use. [Material 3 spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// introduced a set of tone-based surfaces and surface containers in its `ColorScheme`,
    /// which provide more flexibility. The intention is to eventually remove surface tint color
    /// from the framework.
    ///
    /// If this is null, no overlay will be applied. Otherwise this color
    /// will be composited on top of [`color`](Self::color) with an opacity related
    /// to [`elevation`](Self::elevation) and used to paint the background of the card.
    ///
    /// The default is `Colors::TRANSPARENT`.
    ///
    /// See [`Material::surface_tint_color`] for more details on how this
    /// overlay is applied.
    pub surface_tint_color: Option<Color>,

    /// The z-coordinate at which to place this card. This controls the size of
    /// the shadow below the card.
    ///
    /// Defines the card's [`Material::elevation`].
    ///
    /// If this property is null then the ambient [`CardThemeData::elevation`] is
    /// used. If that's null, the default value is 1.0.
    pub elevation: Option<f64>,

    /// The shape of the card's [`Material`].
    ///
    /// Defines the card's [`Material::shape`].
    ///
    /// If null, the ambient [`CardTheme::shape`] from `ThemeData::card_theme` is used.
    /// If that is also null, the shape defaults to a [`RoundedRectangleBorder`].
    /// The default corner radius is 12.0 when `ThemeData::use_material3` is true,
    /// and 4.0 otherwise. For Material 3 outlined cards, the default [`shape`](Self::shape)
    /// also includes a border side (see `OutlinedBorder::side`).
    pub shape: Option<ShapeBorderRef>,

    /// Whether to paint the [`shape`](Self::shape) border in front of the
    /// [`child`](Self::child).
    ///
    /// The default value is true.
    /// If false, the border will be painted behind the [`child`](Self::child).
    pub border_on_foreground: bool,

    /// The content will be clipped (or not) according to this option.
    ///
    /// If this property is null then the ambient [`CardThemeData::clip_behavior`] is
    /// used. If that's null then the behavior will be [`Clip::None`].
    pub clip_behavior: Option<Clip>,

    /// The empty space that surrounds the card.
    ///
    /// Defines the card's outer `Container::margin`.
    ///
    /// If this property is null then the ambient [`CardThemeData::margin`] is used.
    /// If that's null, the default margin is 4.0 logical pixels on
    /// all sides: `EdgeInsets::all(4.0)`.
    pub margin: Option<EdgeInsetsGeometry>,

    /// Whether this widget represents a single semantic container, or if false
    /// a collection of individual semantic nodes.
    ///
    /// Defaults to true.
    ///
    /// Setting this flag to true will attempt to merge all child semantics into
    /// this node. Setting this flag to false will force all child semantic nodes
    /// to be explicit.
    ///
    /// This flag should be false if the card contains multiple different types
    /// of content.
    pub semantic_container: bool,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    variant: CardVariant,
}

impl Card {
    /// Creates an elevated variant of Card.
    ///
    /// Elevated cards have a drop shadow, providing more separation from the
    /// background than filled cards, but less than outlined cards.
    ///
    /// The [`elevation`](Self::elevation) must be null or non-negative.
    pub fn new() -> Card {
        Card::of_variant(CardVariant::Elevated)
    }

    /// Create a filled variant of Card.
    ///
    /// Filled cards provide subtle separation from the background. This has less
    /// emphasis than elevated cards (the default) or outlined cards.
    ///
    /// If `ThemeData::use_material3` is false, this constructor is equivalent to
    /// [`Card::new`].
    pub fn filled() -> Card {
        Card::of_variant(CardVariant::Filled)
    }

    /// Create an outlined variant of Card.
    ///
    /// Outlined cards have a visual boundary around the container. This can
    /// provide greater emphasis than the other types.
    ///
    /// The card's outline is defined by the [`shape`](Self::shape) property. By default, the
    /// card uses a [`RoundedRectangleBorder`] with a 12.0 corner radius, a 1.0
    /// border width, and the color from `ColorScheme::outline_variant`. If you
    /// provide a custom [`shape`](Self::shape), it is recommended to use an `OutlinedBorder`
    /// with a non-null `OutlinedBorder::side` to keep a visible outline.
    ///
    /// If `ThemeData::use_material3` is false, this constructor is equivalent to
    /// [`Card::new`].
    pub fn outlined() -> Card {
        Card::of_variant(CardVariant::Outlined)
    }

    fn of_variant(variant: CardVariant) -> Card {
        Card {
            key: None,
            color: None,
            shadow_color: None,
            surface_tint_color: None,
            elevation: None,
            shape: None,
            border_on_foreground: true,
            clip_behavior: None,
            margin: None,
            semantic_container: true,
            child: None,
            variant,
        }
    }

    /// Dart `Card(key:)`.
    pub fn key(mut self, key: KeyRef) -> Card {
        self.key = Some(key);
        self
    }

    /// Dart `Card(color:)`.
    pub fn color(mut self, color: Color) -> Card {
        self.color = Some(color);
        self
    }

    /// Dart `Card(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> Card {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `Card(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> Card {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `Card(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> Card {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `Card(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> Card {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `Card(borderOnForeground:)`.
    pub fn border_on_foreground(mut self, border_on_foreground: bool) -> Card {
        self.border_on_foreground = border_on_foreground;
        self
    }

    /// Dart `Card(margin:)`.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> Card {
        self.margin = Some(margin);
        self
    }

    /// Dart `Card(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Card {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `Card(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Card {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `Card(semanticContainer:)`.
    pub fn semantic_container(mut self, semantic_container: bool) -> Card {
        self.semantic_container = semantic_container;
        self
    }
}

impl Default for Card {
    fn default() -> Card {
        Card::new()
    }
}

impl StatelessWidget for Card {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let card_theme = CardTheme::of(app, context);
        let defaults: Box<dyn CardThemeDataGetters> = if Theme::of(app, context).use_material3 {
            match self.variant {
                CardVariant::Elevated => Box::new(CardDefaultsM3::new(context)),
                CardVariant::Filled => Box::new(FilledCardDefaultsM3::new(context)),
                CardVariant::Outlined => Box::new(OutlinedCardDefaultsM3::new(context)),
            }
        } else {
            Box::new(CardDefaultsM2::new(context))
        };

        let mut material = Material::new()
            .r#type(MaterialType::Card)
            .elevation(
                self.elevation
                    .or(card_theme.elevation)
                    .or_else(|| defaults.elevation(app))
                    .expect("the defaults give an elevation"),
            )
            .border_on_foreground(self.border_on_foreground)
            .clip_behavior(
                self.clip_behavior
                    .or(card_theme.clip_behavior)
                    .or_else(|| defaults.clip_behavior(app))
                    .expect("the defaults give a clip behavior"),
            );
        material.color = self
            .color
            .or(card_theme.color)
            .or_else(|| defaults.color(app));
        material.shadow_color = self
            .shadow_color
            .or(card_theme.shadow_color)
            .or_else(|| defaults.shadow_color(app));
        material.surface_tint_color = self
            .surface_tint_color
            .or(card_theme.surface_tint_color)
            .or_else(|| defaults.surface_tint_color(app));
        material.shape = self
            .shape
            .clone()
            .or(card_theme.shape)
            .or_else(|| defaults.shape(app));
        let mut child_semantics = Semantics::new().explicit_child_nodes(!self.semantic_container);
        child_semantics.child = self.child.clone();
        material.child = Some(child_semantics.into_widget());

        Semantics::new()
            .container(self.semantic_container)
            .child(
                Padding::new(
                    self.margin
                        .or(card_theme.margin)
                        .or_else(|| defaults.margin(app))
                        .expect("the defaults give a margin"),
                )
                .child(material),
            )
            .into_widget()
    }
}

/// Hand coded defaults based on Material Design 2.
///
/// Dart's `_CardDefaultsM2`.
struct CardDefaultsM2 {
    /// Dart's `super(..)` arguments.
    card_theme_data: CardThemeData,
    context: BuildContext,
}

impl CardDefaultsM2 {
    fn new(context: BuildContext) -> CardDefaultsM2 {
        CardDefaultsM2 {
            card_theme_data: CardThemeData::new()
                .clip_behavior(Clip::None)
                .elevation(1.0)
                .margin(EdgeInsetsGeometry::all(4.0))
                .shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(4.0),
                )),
            context,
        }
    }
}

impl CardThemeDataGetters for CardDefaultsM2 {
    fn card_theme_data(&self) -> &CardThemeData {
        &self.card_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(Theme::of(app, self.context).card_color)
    }

    fn shadow_color(&self, app: &mut App) -> Option<Color> {
        Some(Theme::of(app, self.context).shadow_color)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/card_test.dart`; its semantics cases (`semanticContainer`) wait for
    //! the semantics tree, and its goldens are not ported.

    use waft_embedder::Size;
    use waft_painting::AlignmentGeometry;
    use waft_widgets::{Builder, Center, Container, SizedBox, Text};

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::scaffold::Scaffold;
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app};

    /// What a `Card`'s `Material` was given: Dart's `tester.widget<Material>(find.descendant(
    /// of: find.byType(Card), matching: find.byType(Material)))`.
    pub(crate) struct CardMaterial {
        pub(crate) clip_behavior: Clip,
        pub(crate) elevation: f64,
        pub(crate) color: Option<Color>,
        pub(crate) shadow_color: Option<Color>,
        pub(crate) surface_tint_color: Option<Color>,
        pub(crate) shape: Option<ShapeBorderRef>,
    }

    pub(crate) fn card_material(tester: &WidgetTester) -> CardMaterial {
        let materials =
            find::descendant(find::by_type::<Card>(), find::by_type::<Material>()).evaluate(tester);
        assert_eq!(materials.len(), 1, "one Material under the Card");
        tester.widget(materials[0], |material: &Material| CardMaterial {
            clip_behavior: material.clip_behavior,
            elevation: material.elevation,
            color: material.color,
            shadow_color: material.shadow_color,
            surface_tint_color: material.surface_tint_color,
            shape: material.shape.clone(),
        })
    }

    /// Dart's `tester.widget<Padding>(find.descendant(of: find.byType(Card), matching:
    /// find.byType(Padding)))`, whose padding it returns.
    pub(crate) fn card_padding(tester: &WidgetTester) -> EdgeInsetsGeometry {
        let paddings =
            find::descendant(find::by_type::<Card>(), find::by_type::<Padding>()).evaluate(tester);
        assert_eq!(paddings.len(), 1, "one Padding under the Card");
        tester.widget(paddings[0], |padding: &Padding| padding.padding)
    }

    pub(crate) fn rounded(radius: f64) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::circular(radius),
        )))
    }

    fn scaffold_with(card: Card) -> impl FnOnce(&mut App) -> WidgetRef {
        move |_app| Scaffold::new().body(card).into_widget()
    }

    fn colors_of(theme: fn(&App) -> ThemeData, tester: &WidgetTester) -> ColorScheme {
        theme(&tester.cell.borrow()).color_scheme
    }

    #[test]
    fn material3_card_defaults_elevated_card() {
        test_widgets(|tester| {
            let colors = colors_of(m3, tester);
            pump_app(tester, m3, scaffold_with(Card::new()));

            let padding = card_padding(tester);
            let material = card_material(tester);

            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.elevation, 1.0);
            assert_eq!(padding, EdgeInsetsGeometry::all(4.0));
            assert_eq!(material.color, Some(colors.surface_container_low()));
            assert_eq!(material.shadow_color, Some(colors.shadow()));
            // Don't use surface tint. Toned surface container is used instead.
            assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(material.shape, rounded(12.0));
        });
    }

    #[test]
    fn material3_card_filled_defaults() {
        test_widgets(|tester| {
            let colors = colors_of(m3, tester);
            pump_app(tester, m3, scaffold_with(Card::filled()));

            let padding = card_padding(tester);
            let material = card_material(tester);

            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.elevation, 0.0);
            assert_eq!(padding, EdgeInsetsGeometry::all(4.0));
            assert_eq!(material.color, Some(colors.surface_container_highest()));
            assert_eq!(material.shadow_color, Some(colors.shadow()));
            assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(material.shape, rounded(12.0));
        });
    }

    #[test]
    fn material3_card_outlined_defaults() {
        test_widgets(|tester| {
            let colors = colors_of(m3, tester);
            pump_app(tester, m3, scaffold_with(Card::outlined()));

            let padding = card_padding(tester);
            let material = card_material(tester);

            assert_eq!(material.clip_behavior, Clip::None);
            assert_eq!(material.elevation, 0.0);
            assert_eq!(padding, EdgeInsetsGeometry::all(4.0));
            assert_eq!(material.color, Some(colors.surface));
            assert_eq!(material.shadow_color, Some(colors.shadow()));
            assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                material.shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide {
                        color: colors.outline_variant(),
                        ..BorderSide::default()
                    },
                    BorderRadiusGeometry::circular(12.0),
                )))
            );
        });
    }

    #[test]
    fn material2_card_variants_are_the_elevated_card() {
        test_widgets(|tester| {
            for card in [Card::new(), Card::filled(), Card::outlined()] {
                pump_app(tester, m2, scaffold_with(card));

                let material = card_material(tester);
                assert_eq!(material.elevation, 1.0);
                assert_eq!(material.shape, rounded(4.0));
            }
        });
    }

    #[test]
    fn card_margin() {
        test_widgets(|tester| {
            let contents_key = waft_widgets::GlobalKey::new();
            let contents = |key: &waft_widgets::GlobalKey| {
                Container::new()
                    .key(key.clone().into())
                    .color(Color::new(0xFF00FF00))
                    .width(100.0)
                    .height(100.0)
            };

            tester.pump_widget(
                Container::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(Card::new().child(contents(&contents_key)))
                    .into_widget(),
            );

            // Default margin is 4
            let card = find::by_type::<Card>().evaluate(tester)[0];
            let rect = tester.get_rect(card);
            assert_eq!(rect.top_left(), waft_embedder::Offset::ZERO);
            assert_eq!(rect.size(), Size::new(108.0, 108.0));

            let contents_rect = tester.get_rect(&contents_key);
            assert_eq!(
                contents_rect.top_left(),
                waft_embedder::Offset::new(4.0, 4.0)
            );
            assert_eq!(contents_rect.size(), Size::new(100.0, 100.0));

            tester.pump_widget(
                Container::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(
                        Card::new()
                            .margin(EdgeInsetsGeometry::ZERO)
                            .child(contents(&contents_key)),
                    )
                    .into_widget(),
            );

            // Specified margin is zero
            let card = find::by_type::<Card>().evaluate(tester)[0];
            let rect = tester.get_rect(card);
            assert_eq!(rect.top_left(), waft_embedder::Offset::ZERO);
            assert_eq!(rect.size(), Size::new(100.0, 100.0));

            let contents_rect = tester.get_rect(&contents_key);
            assert_eq!(contents_rect.top_left(), waft_embedder::Offset::ZERO);
            assert_eq!(contents_rect.size(), Size::new(100.0, 100.0));
        });
    }

    #[test]
    fn card_clip_behavior_property_passes_through_to_the_material() {
        test_widgets(|tester| {
            tester.pump_widget(Card::new().into_widget());
            assert_eq!(card_material(tester).clip_behavior, Clip::None);

            tester.pump_widget(Card::new().clip_behavior(Clip::AntiAlias).into_widget());
            assert_eq!(card_material(tester).clip_behavior, Clip::AntiAlias);
        });
    }

    #[test]
    fn card_clip_behavior_property_defers_to_theme_when_null() {
        test_widgets(|tester| {
            tester.pump_widget(
                Builder::new(|app, context| {
                    let theme_data = Theme::of(app, context);
                    let card_theme = theme_data
                        .card_theme
                        .copy_with()
                        .clip_behavior(Clip::AntiAliasWithSaveLayer);
                    Theme::new(
                        theme_data.copy_with().card_theme(card_theme).build(),
                        Card::new(),
                    )
                    .into_widget()
                })
                .into_widget(),
            );
            assert_eq!(
                card_material(tester).clip_behavior,
                Clip::AntiAliasWithSaveLayer
            );
        });
    }

    #[test]
    fn card_shadow_color() {
        test_widgets(|tester| {
            tester.pump_widget(Card::new().into_widget());

            let card = find::by_type::<Card>().evaluate(tester)[0];
            assert_eq!(tester.widget(card, |card: &Card| card.shadow_color), None);
            assert_eq!(
                card_material(tester).shadow_color,
                Some(Color::new(0xFF000000))
            );

            tester.pump_widget(
                Card::new()
                    .shadow_color(Colors::RED.primary())
                    .into_widget(),
            );

            let card = find::by_type::<Card>().evaluate(tester)[0];
            assert_eq!(
                card_material(tester).shadow_color,
                tester.widget(card, |card: &Card| card.shadow_color)
            );
            assert_eq!(
                card_material(tester).shadow_color,
                Some(Colors::RED.primary())
            );
        });
    }

    #[test]
    fn card_renders_at_zero_size() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(
                        SizedBox::shrink()
                            .child(Scaffold::new().body(Card::new().child(Text::new("X")))),
                    )
                    .into_widget()
            });
            let x_text = find::text("X").evaluate(tester)[0];
            let rect = tester.get_rect(x_text);
            assert!(rect.width() <= 0.0 || rect.height() <= 0.0);
        });
    }
}
