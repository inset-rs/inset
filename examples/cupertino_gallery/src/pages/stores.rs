//! A café cart as [`Entity`] stores: the menu and the cart route share one [`Cart`].
//!
//! [`Totals`] `observe`s it. A `read` during `build` rebuilds the widget; a tap `update`s and
//! `notify`s. Neither screen calls `set_state`.

use std::rc::Rc;

use reveal_cupertino::{
    CupertinoButton, CupertinoButtonSize, CupertinoColors, CupertinoDynamicColor, CupertinoIcons,
    CupertinoListSection, CupertinoListTile, CupertinoListTileChevron, CupertinoTheme,
};
use reveal_embedder::{FontFeature, FontWeight, TextAlign};
use reveal_foundation::{App, Entity, Listener, Task};
use reveal_painting::{AnyColor, EdgeInsetsGeometry};
use reveal_rendering::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize};
use reveal_widgets::{
    BuildContext, Column, Icon, IconData, IntoWidget, Padding, Row, SizedBox, StatelessWidget,
    Text, WidgetRef,
};

use crate::app::open_sub_page;
use crate::catalog::Entry;
use crate::support::{badge, body_style, screen, scrolling_body, secondary, section};

pub const SUB_TITLE: &str = "Your Cart";

const LEADING: f64 = 36.0;

struct Product {
    name: &'static str,
    cents: u32,
}

const CATALOG: [Product; 3] = [
    Product {
        name: "Espresso",
        cents: 400,
    },
    Product {
        name: "Pour-over",
        cents: 500,
    },
    Product {
        name: "Croissant",
        cents: 300,
    },
];

fn product_look(index: usize) -> (IconData, AnyColor) {
    match index {
        0 => (CupertinoIcons::flame(), CupertinoColors::SYSTEM_ORANGE),
        1 => (CupertinoIcons::drop(), CupertinoColors::SYSTEM_TEAL),
        _ => (CupertinoIcons::star(), CupertinoColors::SYSTEM_BROWN),
    }
}

#[derive(Clone, Default)]
struct Line {
    product: usize,
    quantity: u32,
}

#[derive(Default)]
struct Cart {
    lines: Vec<Line>,
}

impl Cart {
    fn add(&mut self, product: usize) {
        if let Some(line) = self.lines.iter_mut().find(|line| line.product == product) {
            line.quantity += 1;
            return;
        }
        self.lines.push(Line {
            product,
            quantity: 1,
        });
    }

    fn remove(&mut self, product: usize) {
        if let Some(line) = self.lines.iter_mut().find(|line| line.product == product) {
            line.quantity = line.quantity.saturating_sub(1);
        }
        self.lines.retain(|line| line.quantity > 0);
    }

    fn quantity(&self, product: usize) -> u32 {
        self.lines
            .iter()
            .find(|line| line.product == product)
            .map(|line| line.quantity)
            .unwrap_or(0)
    }

    fn clear(&mut self) {
        self.lines.clear();
    }
}

#[derive(Clone, Copy, Default)]
struct Totals {
    items: u32,
    cents: u32,
}

impl Totals {
    fn of(cart: &Cart) -> Totals {
        let items = cart.lines.iter().map(|line| line.quantity).sum();
        let cents = cart
            .lines
            .iter()
            .map(|line| CATALOG[line.product].cents * line.quantity)
            .sum();
        Totals { items, cents }
    }
}

/// Keeps the [`Entity`] clones so every route of this demo finds the same stores.
#[derive(Default)]
struct ShopBag {
    cart: Option<Entity<Cart>>,
    totals: Option<Entity<Totals>>,
}

fn ensure_shop(app: &mut App) -> (Entity<Cart>, Entity<Totals>) {
    let bag = app.singleton::<ShopBag>();
    if let Some(cart) = app.get(bag).cart.clone() {
        let totals = app
            .get(bag)
            .totals
            .clone()
            .expect("totals is created with the cart");
        return (cart, totals);
    }

    let cart = app.new_entity(|_cx| Cart::default());
    let totals = app.new_entity({
        let cart = cart.clone();
        move |cx| {
            cx.observe(&cart, |totals: &mut Totals, cart, cx| {
                *totals = Totals::of(cart.read(cx));
            })
            .detach();
            Totals::default()
        }
    });

    let shop = app.get_mut(bag);
    shop.cart = Some(cart.clone());
    shop.totals = Some(totals.clone());
    (cart, totals)
}

pub fn body(app: &mut App, _context: BuildContext) -> WidgetRef {
    let (cart, totals) = ensure_shop(app);
    ShopHome { cart, totals }.into_widget()
}

pub fn sub_body(app: &mut App, _context: BuildContext) -> WidgetRef {
    let (cart, totals) = ensure_shop(app);
    YourCart { cart, totals }.into_widget()
}

#[derive(Debug)]
struct ShopHome {
    cart: Entity<Cart>,
    totals: Entity<Totals>,
}

impl StatelessWidget for ShopHome {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let totals = *self.totals.read(app);
        let quantities: [u32; 3] = std::array::from_fn(|i| self.cart.read(app).quantity(i));

        screen(scrolling_body(vec![
            cart_card(totals, app, context),
            section(
                "Menu",
                CATALOG
                    .iter()
                    .enumerate()
                    .map(|(index, product)| {
                        product_row(
                            self.cart.clone(),
                            index,
                            product,
                            quantities[index],
                            app,
                            context,
                        )
                    })
                    .collect(),
            ),
        ]))
    }
}

#[derive(Debug)]
struct YourCart {
    cart: Entity<Cart>,
    totals: Entity<Totals>,
}

impl StatelessWidget for YourCart {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let totals = *self.totals.read(app);
        let lines: Vec<Line> = self.cart.read(app).lines.clone();

        if lines.is_empty() {
            return screen(scrolling_body(vec![empty_cart(app, context)]));
        }

        let mut children: Vec<WidgetRef> = lines
            .iter()
            .map(|line| line_row(self.cart.clone(), line, app, context))
            .collect();
        children.push(total_row(totals, app, context));

        screen(scrolling_body(vec![
            section("Items", children),
            clear_button(self.cart.clone(), app, context),
        ]))
    }
}

fn cart_card(totals: Totals, app: &mut App, context: BuildContext) -> WidgetRef {
    let green = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_GREEN, app, context);
    CupertinoListSection::inset_grouped()
        .children([CupertinoListTile::notched(Text::new("Your cart"))
            .leading(badge(CupertinoIcons::cart_fill(), green, LEADING))
            .leading_size(LEADING)
            .subtitle(secondary(app, context, item_label(totals.items)))
            .additional_info(secondary(app, context, dollars(totals.cents)))
            .trailing(CupertinoListTileChevron::new())
            .on_tap(Rc::new(move |app: &mut App| {
                open_sub_page(app, context, Entry::ShoppingCart, 0);
                Task::ready(())
            }))
            .into_widget()])
        .into_widget()
}

fn product_row(
    cart: Entity<Cart>,
    index: usize,
    product: &Product,
    quantity: u32,
    app: &mut App,
    context: BuildContext,
) -> WidgetRef {
    let (icon, tint) = product_look(index);
    let tint = CupertinoDynamicColor::resolve(&tint, app, context);
    let mut tile = CupertinoListTile::notched(Text::new(product.name))
        .leading(badge(icon, tint, LEADING))
        .leading_size(LEADING)
        .subtitle(secondary(app, context, dollars(product.cents)));
    if quantity == 0 {
        tile = tile
            .trailing(add_label(app, context))
            .on_tap(Rc::new(move |app: &mut App| {
                bump(&cart, app, index, 1);
                Task::ready(())
            }));
    } else {
        tile = tile.trailing(stepper(cart, index, quantity, app, context));
    }
    tile.into_widget()
}

fn line_row(cart: Entity<Cart>, line: &Line, app: &mut App, context: BuildContext) -> WidgetRef {
    let product = &CATALOG[line.product];
    let (icon, tint) = product_look(line.product);
    let tint = CupertinoDynamicColor::resolve(&tint, app, context);
    CupertinoListTile::notched(Text::new(product.name))
        .leading(badge(icon, tint, LEADING))
        .leading_size(LEADING)
        .subtitle(secondary(
            app,
            context,
            dollars(product.cents * line.quantity),
        ))
        .trailing(stepper(cart, line.product, line.quantity, app, context))
        .into_widget()
}

fn total_row(totals: Totals, app: &mut App, context: BuildContext) -> WidgetRef {
    let style = body_style(app, context).font_weight(FontWeight::W600);
    CupertinoListTile::notched(Text::new("Total").style(style.clone()))
        .additional_info(Text::new(dollars(totals.cents)).style(style))
        .into_widget()
}

fn empty_cart(app: &mut App, context: BuildContext) -> WidgetRef {
    let muted = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_GREY, app, context);
    Padding::new(EdgeInsetsGeometry::from_ltrb(24.0, 48.0, 24.0, 24.0))
        .child(
            Column::new()
                .main_axis_alignment(MainAxisAlignment::Start)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .children([
                    Icon::new(Some(CupertinoIcons::cart()))
                        .size(48.0)
                        .color(muted)
                        .into_widget(),
                    SizedBox::new().height(16.0).into_widget(),
                    Text::new("Your cart is empty")
                        .style(body_style(app, context))
                        .into_widget(),
                    SizedBox::new().height(6.0).into_widget(),
                    secondary(app, context, "Add something from the menu."),
                ]),
        )
        .into_widget()
}

fn clear_button(cart: Entity<Cart>, app: &mut App, context: BuildContext) -> WidgetRef {
    let red = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_RED, app, context);
    Padding::new(EdgeInsetsGeometry::from_ltrb(16.0, 8.0, 16.0, 0.0))
        .child(CupertinoButton::new(
            Text::new("Clear cart")
                .style(body_style(app, context).color(red))
                .into_widget(),
            Some(Listener::new(move |app| {
                cart.update(app, |cart, cx| {
                    cart.clear();
                    cx.notify();
                });
            })),
        ))
        .into_widget()
}

fn add_label(app: &mut App, context: BuildContext) -> WidgetRef {
    let color = CupertinoTheme::of(app, context).primary_color();
    Text::new("Add")
        .style(body_style(app, context).color(color))
        .into_widget()
}

fn stepper(
    cart: Entity<Cart>,
    product: usize,
    quantity: u32,
    app: &mut App,
    context: BuildContext,
) -> WidgetRef {
    Row::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .children([
            step_icon(CupertinoIcons::minus_circle(), {
                let cart = cart.clone();
                Listener::new(move |app| bump(&cart, app, product, -1))
            }),
            Padding::new(EdgeInsetsGeometry::symmetric(0.0, 8.0))
                .child(quantity_label(app, context, quantity))
                .into_widget(),
            step_icon(CupertinoIcons::plus_circle_fill(), {
                let cart = cart.clone();
                Listener::new(move |app| bump(&cart, app, product, 1))
            }),
        ])
        .into_widget()
}

/// Two tabular digits, centered. The plus and minus stay put as the count changes.
fn quantity_label(app: &mut App, context: BuildContext, quantity: u32) -> WidgetRef {
    let color = CupertinoDynamicColor::resolve(&CupertinoColors::SECONDARY_LABEL, app, context);
    let style = body_style(app, context)
        .color(color)
        .font_features(vec![FontFeature::enable("tnum")]);
    let width = style.font_size.unwrap_or(17.0) * 1.5;
    SizedBox::new()
        .width(width)
        .child(
            Text::new(quantity.to_string())
                .style(style)
                .text_align(TextAlign::Center)
                .into_widget(),
        )
        .into_widget()
}

fn step_icon(icon: IconData, on_pressed: Listener) -> WidgetRef {
    CupertinoButton::new(
        Icon::new(Some(icon)).size(22.0).into_widget(),
        Some(on_pressed),
    )
    .size_style(CupertinoButtonSize::Small)
    .padding(EdgeInsetsGeometry::all(2.0))
    .into_widget()
}

fn bump(cart: &Entity<Cart>, app: &mut App, product: usize, delta: i32) {
    cart.update(app, |cart, cx| {
        if delta > 0 {
            cart.add(product);
        } else {
            cart.remove(product);
        }
        cx.notify();
    });
}

fn item_label(items: u32) -> String {
    match items {
        0 => "Empty".to_owned(),
        1 => "1 item".to_owned(),
        n => format!("{n} items"),
    }
}

fn dollars(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}
