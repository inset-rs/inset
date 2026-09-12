//! Flutter counterpart: `foundation/constants.dart`.

/// A constant that is true if the application was compiled in release mode.
///
/// More specifically, this is a constant that is true if the application was
/// compiled without `debug_assertions`, which is Cargo's `release` profile
/// default.
///
/// Since this is a const value, it can be used to indicate to the compiler that
/// a particular block of code will not be executed in release mode, and hence
/// can be removed.
///
/// Generally it is better to use [`K_DEBUG_MODE`] or `debug_assert!` to gate
/// code, since using [`K_RELEASE_MODE`] will introduce differences between
/// release and profile builds, which makes performance testing less
/// representative.
///
/// See also:
///
///  * [`K_DEBUG_MODE`], which is true in debug builds.
///  * [`K_PROFILE_MODE`], which is true in profile builds.
pub const K_RELEASE_MODE: bool = !cfg!(debug_assertions);

/// A constant that is true if the application was compiled in profile mode.
///
/// Always false: Cargo has no compiler-visible profile mode.
///
/// See also:
///
///  * [`K_DEBUG_MODE`], which is true in debug builds.
///  * [`K_RELEASE_MODE`], which is true in release builds.
pub const K_PROFILE_MODE: bool = false;

/// A constant that is true if the application was compiled in debug mode.
///
/// More specifically, this is a constant that is true if the application was
/// compiled with `debug_assertions`, which is Cargo's `dev` profile default.
///
/// Since this is a const value, it can be used to indicate to the compiler that
/// a particular block of code will not be executed in debug mode, and hence
/// can be removed.
///
/// An alternative strategy is to use `debug_assert!`, or:
///
/// ```
/// if cfg!(debug_assertions) {
///     // ...debug-only code here...
/// }
/// ```
///
/// See also:
///
///  * [`K_RELEASE_MODE`], which is true in release builds.
///  * [`K_PROFILE_MODE`], which is true in profile builds.
pub const K_DEBUG_MODE: bool = !K_RELEASE_MODE && !K_PROFILE_MODE;

/// The epsilon of tolerable double precision error.
///
/// This is used in various places in the framework to allow for floating point
/// precision loss in calculations. Differences below this threshold are safe to
/// disregard.
pub const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;

/// A constant that is true if the application was compiled to run on the web.
///
/// See also:
///
/// * `default_target_platform`, which is used by themes to find out which
///   platform the application is running on (or, in the case of a web app,
///   which platform the application's browser is running in). Can be overridden
///   in tests with `debug_default_target_platform_override`.
pub const K_IS_WEB: bool = cfg!(target_family = "wasm");

/// A constant that is true if the application was compiled to WebAssembly.
///
/// See also:
///
/// * `default_target_platform`, which is used by themes to find out which
///   platform the application is running on (or, in the case of a web app,
///   which platform the application's browser is running in). Can be overridden
///   in tests with `debug_default_target_platform_override`.
pub const K_IS_WASM: bool = cfg!(target_family = "wasm");
