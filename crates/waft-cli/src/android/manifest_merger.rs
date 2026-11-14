//! The manifest merger: each Android library's manifest merged into the one Waft writes, as
//! Android's Gradle plugin merges a library's into an app's ("Merge multiple manifest files" on
//! developer.android.com), for the elements of [`MANIFEST_CHILDREN`] and below.
//!
//! Waft's manifest is the app's and has the highest priority; the libraries follow in the order
//! the app lists them. An element of a library is matched to one already merged by its match key
//! (`android:name`, or being the only one of its parent) and its attributes are merged into it: an
//! attribute the merged element lacks is added, the same value twice is one, and two values are a
//! conflict, which is refused, as Gradle refuses it until the app adds a merge rule marker. An
//! `android:required` is merged by *or*. An `<intent-filter>`, an `<intent>` and a `<provider>`
//! of `<queries>` are kept as they are, beside any already there. The `<manifest>` element's own
//! attributes are the app's alone. A library's `<uses-sdk>` is not merged: it is refused when its
//! `minSdkVersion` is above the app's, and a `targetSdkVersion` below a level that restricted an
//! API adds the permission that keeps the API working, as Android's table of implicit system
//! permissions says.
//!
//! Before a library's manifest is merged, its `${applicationId}` placeholders become the app's
//! package name, its relative class names (`.Receiver`) become full ones under the library's
//! package, and its lint-only `tools:` attributes are dropped.
//!
//! What it does not merge it refuses, naming the library and the element: an element outside the
//! stated set, a `tools:` merge rule marker (the app writes no manifest of its own to put one in),
//! a placeholder other than `${applicationId}`, and a conflict. A later editor who widens the set
//! adds a [`Rule`] with the match key and the merge policy Android's table of merge policies gives
//! the element, never a way to let an element through unmerged.

use anyhow::{Context, Result, bail};

use super::xml::{self, Attribute, Element};

/// The manifest of one Android library, and how the app names the library.
pub struct LibraryManifest<'a> {
    /// The library's path as the app lists it, which every refusal names.
    pub library: &'a str,
    pub manifest: &'a Element,
}

/// Who wrote what the merged manifest holds before any library is merged.
const APP_SOURCE: &str = "Waft's manifest";

/// How an element is matched to one already merged: Android's match keys.
#[derive(Clone, Copy)]
enum Key {
    /// `android:name`.
    Name,
    /// `android:name`, or `android:glEsVersion` where it has none: `<uses-feature>`'s.
    NameOrGlEsVersion,
    /// None: there is one of the element in its parent.
    Single,
}

/// What happens to an element of a library: Android's merge policies.
#[derive(Clone, Copy)]
enum Policy {
    /// Matched by its key and merged attribute by attribute, its children by their own rules.
    Merge { key: Key, children: &'static [Rule] },
    /// Added as it is, beside whatever is there.
    Keep,
}

/// The policy for the element of one name in its parent.
#[derive(Clone, Copy)]
struct Rule {
    element: &'static str,
    policy: Policy,
}

const fn merged_by(element: &'static str, key: Key, children: &'static [Rule]) -> Rule {
    Rule {
        element,
        policy: Policy::Merge { key, children },
    }
}

const fn kept(element: &'static str) -> Rule {
    Rule {
        element,
        policy: Policy::Keep,
    }
}

/// The children of an activity, an alias, a service, a receiver and a provider.
const COMPONENT_CHILDREN: &[Rule] = &[
    kept("intent-filter"),
    merged_by("meta-data", Key::Name, &[]),
];

const APPLICATION_CHILDREN: &[Rule] = &[
    merged_by("activity", Key::Name, COMPONENT_CHILDREN),
    merged_by("activity-alias", Key::Name, COMPONENT_CHILDREN),
    merged_by("service", Key::Name, COMPONENT_CHILDREN),
    merged_by("receiver", Key::Name, COMPONENT_CHILDREN),
    merged_by("provider", Key::Name, COMPONENT_CHILDREN),
    merged_by("meta-data", Key::Name, &[]),
    merged_by("uses-library", Key::Name, &[]),
    merged_by("uses-native-library", Key::Name, &[]),
];

const QUERIES_CHILDREN: &[Rule] = &[
    merged_by("package", Key::Name, &[]),
    kept("intent"),
    kept("provider"),
];

/// The elements of a library's `<manifest>` that are merged, besides `<uses-sdk>`, which is
/// checked.
const MANIFEST_CHILDREN: &[Rule] = &[
    merged_by("uses-permission", Key::Name, &[]),
    merged_by("uses-permission-sdk-23", Key::Name, &[]),
    merged_by("permission", Key::Name, &[]),
    merged_by("uses-feature", Key::NameOrGlEsVersion, &[]),
    merged_by("queries", Key::Single, QUERIES_CHILDREN),
    merged_by("application", Key::Single, APPLICATION_CHILDREN),
];

/// The elements whose `android:required` is merged by *or*: what one manifest requires, the app
/// requires.
const REQUIRED_BY_ANY: [&str; 3] = ["uses-feature", "uses-library", "uses-native-library"];

/// The `tools:` attributes that only lint reads, dropped from a library's manifest.
const LINT_ATTRIBUTES: [&str; 3] = ["tools:ignore", "tools:targetApi", "tools:locale"];

/// The attributes that name a class, by the element that carries them, which Android resolves
/// against the manifest's package when the name is relative.
const CLASS_NAME_ATTRIBUTES: [(&str, &str); 9] = [
    ("application", "android:name"),
    ("application", "android:backupAgent"),
    ("application", "android:manageSpaceActivity"),
    ("activity", "android:name"),
    ("activity-alias", "android:name"),
    ("activity-alias", "android:targetActivity"),
    ("service", "android:name"),
    ("receiver", "android:name"),
    ("provider", "android:name"),
];

/// The only placeholder a build defines whatever the app configures.
const APPLICATION_ID_PLACEHOLDER: &str = "applicationId";

/// Waft's manifest, `app`, with each of `libraries` merged into it in turn.
pub fn merge(app: &str, libraries: &[LibraryManifest]) -> Result<String> {
    let app = xml::parse(app).context("Waft's own manifest does not parse")?;
    let application_id = app
        .attribute("package")
        .context("Waft's manifest names no package")?
        .to_owned();
    let app_sdk = UsesSdk::of(&app).context("Waft's manifest has no numeric <uses-sdk>")?;
    let mut merged = Node::from_element(&app, APP_SOURCE);
    for library in libraries {
        merge_library(&mut merged, library, &application_id, app_sdk)
            .with_context(|| format!("{}: its manifest cannot be merged", library.library))?;
    }
    Ok(xml::write(&merged.into_element()))
}

/// The library's package, by which its relative class names and its R class are named.
pub fn package(manifest: &Element) -> Result<&str> {
    if manifest.name != "manifest" {
        bail!("its root element is <{}>, not <manifest>", manifest.name);
    }
    manifest
        .attribute("package")
        .context("its <manifest> names no package")
}

fn merge_library<'a>(
    merged: &mut Node<'a>,
    library: &LibraryManifest<'a>,
    application_id: &str,
    app_sdk: UsesSdk,
) -> Result<()> {
    let package = package(library.manifest)?;
    let prepared = prepare(library.manifest, package, application_id)?;
    let target_sdk = check_uses_sdk(&prepared, app_sdk)?;
    for child in prepared
        .children
        .iter()
        .filter(|child| child.name != "uses-sdk")
    {
        merge_child(merged, child, MANIFEST_CHILDREN, library.library)?;
    }
    for permission in implied_permissions(target_sdk, app_sdk.target, merged) {
        let mut uses_permission = Element::new("uses-permission");
        uses_permission.attributes.push(Attribute {
            name: "android:name".to_owned(),
            value: permission.to_owned(),
        });
        merge_child(merged, &uses_permission, MANIFEST_CHILDREN, library.library)?;
    }
    Ok(())
}

/// `element` and its children with lint's attributes dropped, `${applicationId}` replaced and
/// relative class names made full under `package`; refused for a merge rule marker or another
/// placeholder.
fn prepare(element: &Element, package: &str, application_id: &str) -> Result<Element> {
    let mut prepared = Element::new(&element.name);
    for attribute in &element.attributes {
        if is_dropped_tools_attribute(attribute) {
            continue;
        }
        if attribute.name.starts_with("tools:") {
            bail!(
                "{} carries {}=\"{}\", a merge rule marker, which Waft's manifest merger does not apply",
                describe_element(element),
                attribute.name,
                attribute.value
            );
        }
        let value = replace_placeholders(&attribute.value, application_id)
            .with_context(|| format!("{} {}", describe_element(element), attribute.name))?;
        let value = if names_a_class(&element.name, &attribute.name) {
            full_class_name(&value, package)
        } else {
            value
        };
        prepared.attributes.push(Attribute {
            name: attribute.name.clone(),
            value,
        });
    }
    for child in &element.children {
        prepared
            .children
            .push(prepare(child, package, application_id)?);
    }
    Ok(prepared)
}

/// Whether the manifest merger drops `attribute` without effect: one lint reads, or the default
/// marker `tools:node="merge"`.
fn is_dropped_tools_attribute(attribute: &Attribute) -> bool {
    LINT_ATTRIBUTES.contains(&attribute.name.as_str())
        || (attribute.name == "tools:node" && attribute.value == "merge")
}

/// `value` with each `${applicationId}` replaced by `application_id`; refused for any other
/// placeholder, which only Gradle's `manifestPlaceholders` would define.
fn replace_placeholders(value: &str, application_id: &str) -> Result<String> {
    let mut replaced = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let end = rest[start..]
            .find('}')
            .map(|end| start + end)
            .with_context(|| format!("`{value}` opens a placeholder it does not close"))?;
        let name = &rest[start + 2..end];
        if name != APPLICATION_ID_PLACEHOLDER {
            bail!(
                "`{value}` holds the placeholder ${{{name}}}; only ${{{APPLICATION_ID_PLACEHOLDER}}} is defined"
            );
        }
        replaced.push_str(&rest[..start]);
        replaced.push_str(application_id);
        rest = &rest[end + 1..];
    }
    replaced.push_str(rest);
    Ok(replaced)
}

fn names_a_class(element: &str, attribute: &str) -> bool {
    CLASS_NAME_ATTRIBUTES.contains(&(element, attribute))
}

/// A class name as Android resolves it against `package`: `.Receiver` and `Receiver` are
/// `<package>.Receiver`; a name with a dot inside is full already.
fn full_class_name(name: &str, package: &str) -> String {
    if name.starts_with('.') {
        format!("{package}{name}")
    } else if !name.contains('.') {
        format!("{package}.{name}")
    } else {
        name.to_owned()
    }
}

/// A manifest's `<uses-sdk>` levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UsesSdk {
    min: u32,
    target: u32,
}

impl UsesSdk {
    /// The levels `manifest` declares, as Android reads them: `minSdkVersion` defaults to 1 and
    /// `targetSdkVersion` to `minSdkVersion`.
    fn of(manifest: &Element) -> Result<UsesSdk> {
        let uses_sdk = manifest
            .children
            .iter()
            .find(|child| child.name == "uses-sdk");
        let level = |name: &str| -> Result<Option<u32>> {
            let Some(value) = uses_sdk.and_then(|uses_sdk| uses_sdk.attribute(name)) else {
                return Ok(None);
            };
            value.parse().map(Some).with_context(|| {
                format!("<uses-sdk> {name}=\"{value}\" is not an API level but a codename")
            })
        };
        let min = level("android:minSdkVersion")?.unwrap_or(1);
        let target = level("android:targetSdkVersion")?.unwrap_or(min);
        Ok(UsesSdk { min, target })
    }
}

/// The library's `targetSdkVersion`; refused when its `minSdkVersion` is above the app's.
fn check_uses_sdk(library: &Element, app: UsesSdk) -> Result<u32> {
    let sdk = UsesSdk::of(library)?;
    if sdk.min > app.min {
        bail!(
            "uses-sdk:minSdkVersion {} cannot be smaller than version {} declared in the library: \
             raise `min_sdk` under [package.metadata.waft.android] to {}",
            app.min,
            sdk.min,
            sdk.min
        );
    }
    Ok(sdk.target)
}

/// The permissions a library that targets `library_target` is granted implicitly, which an app
/// that targets `app_target` must ask for in its place: Android's table of implicit system
/// permissions.
fn implied_permissions(library_target: u32, app_target: u32, merged: &Node) -> Vec<&'static str> {
    let mut implied = Vec::new();
    if library_target <= 3 && app_target > 3 {
        implied.extend([
            "android.permission.WRITE_EXTERNAL_STORAGE",
            "android.permission.READ_PHONE_STATE",
        ]);
    }
    if library_target <= 15 && app_target > 15 {
        let asks_for = |permission: &str| {
            merged.children.iter().any(|child| {
                child.name == "uses-permission"
                    && child.attribute("android:name") == Some(permission)
            })
        };
        if asks_for("android.permission.READ_CONTACTS") {
            implied.push("android.permission.READ_CALL_LOG");
        }
        if asks_for("android.permission.WRITE_CONTACTS") {
            implied.push("android.permission.WRITE_CALL_LOG");
        }
    }
    implied
}

/// Merges `child`, an element of the library `source`, into `parent` by the rule `rules` give
/// its name; refused for an element they do not name.
fn merge_child<'a>(
    parent: &mut Node<'a>,
    child: &Element,
    rules: &[Rule],
    source: &'a str,
) -> Result<()> {
    let Some(rule) = rules.iter().find(|rule| rule.element == child.name) else {
        bail!(
            "{} in {} is not an element Waft's manifest merger merges",
            describe_element(child),
            parent.describe()
        );
    };
    match rule.policy {
        Policy::Keep => parent.children.push(Node::from_element(child, source)),
        Policy::Merge { key, children } => {
            let key = match_key(child, key)?;
            let index = match parent.children.iter().position(|merged| {
                merged.name == child.name
                    && key.is_none_or(|(name, value)| merged.attribute(name) == Some(value))
            }) {
                Some(index) => index,
                None => {
                    parent.children.push(Node::new(&child.name));
                    parent.children.len() - 1
                }
            };
            merge_element(&mut parent.children[index], child, children, source)?;
        }
    }
    Ok(())
}

/// The attribute and value `element` is matched by, `None` for an element of which there is one.
fn match_key(element: &Element, key: Key) -> Result<Option<(&'static str, &str)>> {
    let keyed_by = |name: &'static str| element.attribute(name).map(|value| (name, value));
    let found = match key {
        Key::Single => return Ok(None),
        Key::Name => keyed_by("android:name"),
        Key::NameOrGlEsVersion => {
            keyed_by("android:name").or_else(|| keyed_by("android:glEsVersion"))
        }
    };
    found
        .map(Some)
        .with_context(|| format!("{} has no android:name", describe_element(element)))
}

fn merge_element<'a>(
    merged: &mut Node<'a>,
    element: &Element,
    children: &[Rule],
    source: &'a str,
) -> Result<()> {
    for attribute in &element.attributes {
        merged.merge_attribute(attribute, source)?;
    }
    for child in &element.children {
        merge_child(merged, child, children, source)?;
    }
    Ok(())
}

/// An element of the merged manifest, each attribute with the manifest it came from.
struct Node<'a> {
    name: String,
    attributes: Vec<SourcedAttribute<'a>>,
    children: Vec<Node<'a>>,
}

struct SourcedAttribute<'a> {
    name: String,
    value: String,
    source: &'a str,
}

impl<'a> Node<'a> {
    fn new(name: &str) -> Node<'a> {
        Node {
            name: name.to_owned(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    fn from_element(element: &Element, source: &'a str) -> Node<'a> {
        Node {
            name: element.name.clone(),
            attributes: element
                .attributes
                .iter()
                .map(|attribute| SourcedAttribute {
                    name: attribute.name.clone(),
                    value: attribute.value.clone(),
                    source,
                })
                .collect(),
            children: element
                .children
                .iter()
                .map(|child| Node::from_element(child, source))
                .collect(),
        }
    }

    fn into_element(self) -> Element {
        Element {
            name: self.name,
            attributes: self
                .attributes
                .into_iter()
                .map(|attribute| Attribute {
                    name: attribute.name,
                    value: attribute.value,
                })
                .collect(),
            children: self.children.into_iter().map(Node::into_element).collect(),
        }
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }

    /// Adds `attribute` from `source`, unless this element has it: the same value is kept, an
    /// `android:required` of [`REQUIRED_BY_ANY`] is *or*ed, and another value is a conflict.
    fn merge_attribute(&mut self, attribute: &Attribute, source: &'a str) -> Result<()> {
        let required_by_any =
            attribute.name == "android:required" && REQUIRED_BY_ANY.contains(&self.name.as_str());
        let element = self.describe();
        let Some(merged) = self
            .attributes
            .iter_mut()
            .find(|merged| merged.name == attribute.name)
        else {
            self.attributes.push(SourcedAttribute {
                name: attribute.name.clone(),
                value: attribute.value.clone(),
                source,
            });
            return Ok(());
        };
        if merged.value == attribute.value {
            return Ok(());
        }
        if required_by_any {
            merged.value = "true".to_owned();
            return Ok(());
        }
        bail!(
            "{element} {}=\"{}\" clashes with the value \"{}\" {} gives it",
            attribute.name,
            attribute.value,
            merged.value,
            merged.source
        )
    }

    /// `<receiver android:name="…">`, or `<application>` for an element without a name.
    fn describe(&self) -> String {
        describe(&self.name, self.attribute("android:name"))
    }
}

fn describe_element(element: &Element) -> String {
    describe(&element.name, element.attribute("android:name"))
}

fn describe(element: &str, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("<{element} android:name=\"{name}\">"),
        None => format!("<{element}>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.example.app">
  <uses-sdk android:minSdkVersion="31" android:targetSdkVersion="34" />
  <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
  <application android:label="App" android:theme="@android:style/Theme.DeviceDefault.NoActionBar">
    <activity android:name="rs.waft.WaftActivity" android:exported="true">
      <intent-filter><action android:name="android.intent.action.MAIN" /></intent-filter>
    </activity>
  </application>
  <queries><intent><action android:name="android.intent.action.VIEW" /></intent></queries>
</manifest>"#;

    fn library(body: &str) -> Element {
        xml::parse(&format!(
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" xmlns:tools="http://schemas.android.com/tools" package="com.example.lib">{body}</manifest>"#
        ))
        .unwrap()
    }

    fn merged(libraries: &[(&str, Element)]) -> Result<Element> {
        let manifests: Vec<_> = libraries
            .iter()
            .map(|(library, manifest)| LibraryManifest { library, manifest })
            .collect();
        Ok(xml::parse(&merge(APP, &manifests)?).unwrap())
    }

    fn refusal(libraries: &[(&str, Element)]) -> String {
        format!("{:#}", merged(libraries).unwrap_err())
    }

    fn child<'a>(parent: &'a Element, name: &str) -> &'a Element {
        parent
            .children
            .iter()
            .find(|child| child.name == name)
            .unwrap_or_else(|| panic!("no <{name}>"))
    }

    fn named<'a>(parent: &'a Element, element: &str, name: &str) -> Vec<&'a Element> {
        parent
            .children
            .iter()
            .filter(|child| child.name == element && child.attribute("android:name") == Some(name))
            .collect()
    }

    #[test]
    fn a_librarys_permissions_application_attributes_and_components_are_merged() {
        let lib = library(
            r#"<uses-sdk android:minSdkVersion="22" />
            <uses-permission android:name="android.permission.INTERNET" />
            <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
            <application android:networkSecurityConfig="@xml/network_security_config" tools:ignore="UnusedAttribute">
              <meta-data android:name="com.example.lib.key" android:value="v" />
              <receiver android:name=".Receiver" android:exported="false" />
            </application>"#,
        );
        let manifest = merged(&[("android/lib.aar", lib)]).unwrap();
        assert_eq!(
            named(&manifest, "uses-permission", "android.permission.INTERNET").len(),
            1
        );
        assert_eq!(
            named(
                &manifest,
                "uses-permission",
                "android.permission.POST_NOTIFICATIONS"
            )
            .len(),
            1,
            "a permission both ask for is asked for once"
        );
        let application = child(&manifest, "application");
        assert_eq!(
            application.attribute("android:networkSecurityConfig"),
            Some("@xml/network_security_config")
        );
        assert_eq!(application.attribute("android:label"), Some("App"));
        assert!(
            !application
                .attributes
                .iter()
                .any(|attribute| attribute.name.starts_with("tools:")),
            "lint's attributes never reach the package"
        );
        assert_eq!(
            named(application, "meta-data", "com.example.lib.key")[0].attribute("android:value"),
            Some("v")
        );
        assert_eq!(
            named(application, "receiver", "com.example.lib.Receiver").len(),
            1,
            "a relative class name is the library's package's"
        );
        assert_eq!(
            child(&manifest, "uses-sdk").attribute("android:minSdkVersion"),
            Some("31"),
            "the app's levels stand"
        );
    }

    #[test]
    fn the_same_component_in_two_libraries_merges_its_attributes_and_keeps_both_filters() {
        let first = library(
            r#"<application><service android:name="com.example.Shared" android:exported="false">
              <intent-filter><action android:name="a" /></intent-filter>
            </service></application>"#,
        );
        let second = library(
            r#"<application><service android:name="com.example.Shared" android:enabled="true">
              <intent-filter><action android:name="b" /></intent-filter>
            </service></application>"#,
        );
        let manifest = merged(&[("a.aar", first), ("b.aar", second)]).unwrap();
        let services = named(
            child(&manifest, "application"),
            "service",
            "com.example.Shared",
        );
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].attribute("android:exported"), Some("false"));
        assert_eq!(services[0].attribute("android:enabled"), Some("true"));
        assert_eq!(
            services[0]
                .children
                .iter()
                .filter(|child| child.name == "intent-filter")
                .count(),
            2
        );
    }

    #[test]
    fn queries_and_features_merge_into_the_apps() {
        let lib = library(
            r#"<uses-feature android:name="android.hardware.camera" android:required="false" />
            <queries><package android:name="com.example.other" /></queries>"#,
        );
        let requiring = library(
            r#"<uses-feature android:name="android.hardware.camera" android:required="true" />"#,
        );
        let manifest = merged(&[("a.aar", lib), ("b.aar", requiring)]).unwrap();
        let queries: Vec<_> = manifest
            .children
            .iter()
            .filter(|child| child.name == "queries")
            .collect();
        assert_eq!(queries.len(), 1, "one <queries>, the app's");
        assert_eq!(queries[0].children.len(), 2);
        assert_eq!(
            named(&manifest, "uses-feature", "android.hardware.camera")[0]
                .attribute("android:required"),
            Some("true"),
            "what one manifest requires the app requires"
        );
    }

    #[test]
    fn the_application_id_placeholder_is_the_apps_package() {
        let lib = library(
            r#"<permission android:name="${applicationId}.LIB" />
            <application><provider android:name=".Provider" android:authorities="${applicationId}.lib" /></application>"#,
        );
        let manifest = merged(&[("lib.aar", lib)]).unwrap();
        assert_eq!(
            named(&manifest, "permission", "com.example.app.LIB").len(),
            1
        );
        let provider = &named(
            child(&manifest, "application"),
            "provider",
            "com.example.lib.Provider",
        )[0];
        assert_eq!(
            provider.attribute("android:authorities"),
            Some("com.example.app.lib")
        );
    }

    #[test]
    fn a_library_without_a_target_level_is_granted_androids_implicit_permissions() {
        let manifest = merged(&[("old.jar", library(""))]).unwrap();
        assert_eq!(
            named(
                &manifest,
                "uses-permission",
                "android.permission.WRITE_EXTERNAL_STORAGE"
            )
            .len(),
            1,
            "no <uses-sdk> targets API 1"
        );
        let contacts = library(
            r#"<uses-sdk android:minSdkVersion="9" /><uses-permission android:name="android.permission.READ_CONTACTS" />"#,
        );
        let manifest = merged(&[("contacts.aar", contacts)]).unwrap();
        assert_eq!(
            named(
                &manifest,
                "uses-permission",
                "android.permission.READ_CALL_LOG"
            )
            .len(),
            1
        );
        assert!(
            named(
                &manifest,
                "uses-permission",
                "android.permission.READ_PHONE_STATE"
            )
            .is_empty()
        );
    }

    #[test]
    fn two_values_for_one_attribute_are_refused_naming_both_sources() {
        let first = library(r#"<application android:networkSecurityConfig="@xml/a" />"#);
        let second = library(r#"<application android:networkSecurityConfig="@xml/b" />"#);
        let refused = refusal(&[("android/a.aar", first), ("android/b.aar", second)]);
        assert!(refused.starts_with("android/b.aar: "), "{refused}");
        assert!(refused.contains("<application>"), "{refused}");
        assert!(
            refused.contains("@xml/b") && refused.contains("@xml/a"),
            "{refused}"
        );
        assert!(refused.contains("android/a.aar gives it"), "{refused}");

        let theme = library(r#"<application android:theme="@style/Lib" />"#);
        assert!(refusal(&[("lib.aar", theme)]).contains("Waft's manifest gives it"));
    }

    #[test]
    fn an_element_outside_the_stated_set_is_refused() {
        let instrumentation = library(r#"<instrumentation android:name=".Test" />"#);
        let refused = refusal(&[("lib.aar", instrumentation)]);
        assert!(refused.starts_with("lib.aar: "), "{refused}");
        assert!(refused.contains("<instrumentation"), "{refused}");
        let nested = library(
            r#"<application><provider android:name=".P"><grant-uri-permission android:path="/" /></provider></application>"#,
        );
        assert!(refusal(&[("lib.aar", nested)]).contains("<grant-uri-permission>"));
    }

    #[test]
    fn merge_rule_markers_and_unknown_placeholders_are_refused() {
        let marker = library(
            r#"<application><activity android:name=".A" tools:node="remove" /></application>"#,
        );
        assert!(refusal(&[("lib.aar", marker)]).contains("tools:node=\"remove\""));
        let placeholder = library(
            r#"<application><meta-data android:name="k" android:value="${apiKey}" /></application>"#,
        );
        assert!(refusal(&[("lib.aar", placeholder)]).contains("${apiKey}"));
        let default_marker = library(r#"<application tools:node="merge" />"#);
        assert!(merged(&[("lib.aar", default_marker)]).is_ok());
    }

    #[test]
    fn a_library_needing_a_later_android_is_refused() {
        let later = library(r#"<uses-sdk android:minSdkVersion="33" />"#);
        let refused = refusal(&[("lib.aar", later)]);
        assert!(refused.contains("minSdkVersion 31"), "{refused}");
        assert!(refused.contains("min_sdk"), "{refused}");
    }

    #[test]
    fn an_element_without_its_match_key_is_refused() {
        let unnamed = library(r#"<application><meta-data android:value="v" /></application>"#);
        assert!(refusal(&[("lib.aar", unnamed)]).contains("has no android:name"));
    }

    #[test]
    fn class_names_resolve_as_android_resolves_them() {
        assert_eq!(full_class_name(".A", "p.q"), "p.q.A");
        assert_eq!(full_class_name("A", "p.q"), "p.q.A");
        assert_eq!(full_class_name("x.y.A", "p.q"), "x.y.A");
    }
}
