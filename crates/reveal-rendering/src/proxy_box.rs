//! Flutter counterpart: `rendering/proxy_box.dart` (`HitTestBehavior` only).

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
