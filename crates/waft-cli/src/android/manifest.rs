//! The package's `AndroidManifest.xml`: Waft's activities, provider and receiver, the app's
//! permissions, devices and file types, the links the opener asks about, merged with the manifests
//! of the Android libraries the app depends on (`manifest_merger.rs`), as Gradle merges them.

use anyhow::Result;

use super::{AndroidLibrary, LibraryManifest, manifest_merger};

use crate::project::{FileAssociation, UsageDescriptions};
use crate::usage_descriptions;

/// Waft's activity: the platform's `NativeActivity`, which loads a native library and calls its
/// `ANativeActivity_onCreate`, Waft's Android host's, handing activity results on to it.
pub(super) const WAFT_ACTIVITY: &str = "rs.waft.WaftActivity";

/// Waft's activity for a view the app asks for, each in a task of its own.
const WAFT_DOCUMENT_ACTIVITY: &str = "rs.waft.WaftDocumentActivity";

/// The most tasks of the document activity Recents keeps before it removes the oldest, which
/// finishes its activity as a swipe does: the most Android takes, where its default is 16.
const MAX_RECENTS: u32 = 50;

/// Waft's file provider, which shares a file of the app's with the app that opens it.
const WAFT_FILE_PROVIDER: &str = "rs.waft.WaftFileProvider";

/// Waft's notification receiver, which builds the app's notifications and hears their alarms.
const WAFT_NOTIFICATION_RECEIVER: &str = "rs.waft.WaftNotificationReceiver";

/// What every app may do for its notifications: post them, which Android 13 asks the user for, and
/// show one at its exact time, which Android 14 leaves to the user to grant in the app's settings.
const PERMISSIONS: [&str; 2] = [
    "android.permission.POST_NOTIFICATIONS",
    "android.permission.SCHEDULE_EXACT_ALARM",
];

/// The schemes of the links whose viewers the app may ask about (`<queries>`): from Android 11 the
/// system answers an app only about the intents its manifest names. The application's meta-data
/// lists them for the app's native code under [`QUERIED_SCHEMES_KEY`], so it knows which answers
/// are the system's whole answer.
const QUERIED_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

const QUERIED_SCHEMES_KEY: &str = "rs.waft.queried_schemes";

/// Every configuration change the activity absorbs rather than being recreated for, which is
/// every one the activity element's reference names: the host hears each as a change of the
/// activity's configuration and of its window, and draws the next frame for it. An activity
/// Android recreates all the same, as under "Don't keep activities", hands its view to the one made
/// in its place (`waft-embedder-android`'s `saved_view.rs`), so the app runs on either way; this
/// spares the window that goes and comes back.
const CONFIG_CHANGES: &str = "colorMode|density|fontScale|fontWeightAdjustment|grammaticalGender|keyboard|keyboardHidden|layoutDirection|locale|mcc|mnc|navigation|orientation|screenLayout|screenSize|smallestScreenSize|touchscreen|uiMode";

pub(super) struct Manifest<'a> {
    pub(super) package: &'a str,
    pub(super) label: &'a str,
    /// The `cdylib`'s name, which the activity loads as `lib<name>.so`.
    pub(super) library: &'a str,
    pub(super) version_code: u32,
    pub(super) version_name: &'a str,
    pub(super) min_sdk: u32,
    pub(super) target_sdk: u32,
    pub(super) debuggable: bool,
    pub(super) has_icon: bool,
    pub(super) category: &'a str,
    /// The kinds of file the system offers the app for.
    pub(super) file_associations: &'a [FileAssociation],
    /// The permissions the app asks for besides [`PERMISSIONS`].
    pub(super) permissions: &'a [String],
    /// The devices the app declares it opens, whose permissions and features it asks for.
    pub(super) devices: &'a UsageDescriptions,
}

/// The manifest around Waft's activity. `configChanges` keeps the activity through a rotation or
/// a switch to dark mode, which the host hears as a change of configuration and a resize;
/// `adjustResize`, which Flutter's template sets too, has the system leave the window in
/// place under the keyboard, which the host hears of as the window's insets. The launcher's
/// activity is `singleTask`, the one task files, links and notification taps reach; beside it the
/// document activity, not exported, starts a task of its own for each view the app asks for
/// (`standard` and `intoExisting`, as Android requires of an activity started as a new document),
/// with the same `configChanges`, Recents keeping [`MAX_RECENTS`] of them. Waft's file provider is declared at `<package>.waft.files`, not
/// exported, granting each URI it shares; its notification receiver, not exported, hears only the
/// app's own alarms; `<queries>` names viewing the links of [`QUERIED_SCHEMES`]; the app asks for
/// [`PERMISSIONS`] and its own; the activity is offered for the files of the app's associations
/// ([`file_intent_filters`]); and it asks for the permission of each device it declares, using its
/// feature, not required ([`uses_features`]).
pub(super) fn manifest(manifest: &Manifest) -> String {
    let icon = if manifest.has_icon {
        r#" android:icon="@mipmap/ic_launcher""#
    } else {
        ""
    };
    let debuggable = if manifest.debuggable {
        r#" android:debuggable="true""#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package}"
    android:versionCode="{version_code}"
    android:versionName="{version_name}">
  <uses-sdk android:minSdkVersion="{min_sdk}" android:targetSdkVersion="{target_sdk}" />
{permissions}{features}  <application
      android:label="{label}"{icon}{debuggable}
      android:appCategory="{category}"
      android:hasCode="true"
      android:extractNativeLibs="false"
      android:theme="@android:style/Theme.DeviceDefault.NoActionBar">
    <activity
        android:name="{activity}"
        android:exported="true"
        android:launchMode="singleTask"
        android:configChanges="{config_changes}"
        android:windowSoftInputMode="adjustResize">
      <meta-data android:name="android.app.lib_name" android:value="{library}" />
      <intent-filter>
        <action android:name="android.intent.action.MAIN" />
        <category android:name="android.intent.category.LAUNCHER" />
      </intent-filter>
{file_intent_filters}    </activity>
    <activity
        android:name="{document_activity}"
        android:exported="false"
        android:launchMode="standard"
        android:documentLaunchMode="intoExisting"
        android:maxRecents="{max_recents}"
        android:configChanges="{config_changes}"
        android:windowSoftInputMode="adjustResize">
      <meta-data android:name="android.app.lib_name" android:value="{library}" />
    </activity>
    <provider
        android:name="{provider}"
        android:authorities="{package}.waft.files"
        android:exported="false"
        android:grantUriPermissions="true" />
    <receiver android:name="{receiver}" android:exported="false" />
    <meta-data android:name="{queried_schemes_key}" android:value="{queried_schemes}" />
  </application>
  <queries>
{queries}  </queries>
</manifest>
"#,
        config_changes = CONFIG_CHANGES,
        package = manifest.package,
        version_code = manifest.version_code,
        version_name = xml_escape(manifest.version_name),
        min_sdk = manifest.min_sdk,
        target_sdk = manifest.target_sdk,
        label = xml_escape(manifest.label),
        category = manifest.category,
        activity = WAFT_ACTIVITY,
        document_activity = WAFT_DOCUMENT_ACTIVITY,
        max_recents = MAX_RECENTS,
        library = manifest.library,
        provider = WAFT_FILE_PROVIDER,
        receiver = WAFT_NOTIFICATION_RECEIVER,
        permissions = uses_permissions(
            &[
                manifest.permissions,
                &usage_descriptions::android_permissions(manifest.devices),
            ]
            .concat()
        ),
        features = uses_features(&usage_descriptions::android_features(manifest.devices)),
        queried_schemes_key = QUERIED_SCHEMES_KEY,
        queried_schemes = QUERIED_SCHEMES.join(","),
        queries = queried_intents(),
        file_intent_filters = file_intent_filters(manifest.file_associations),
    )
}

/// The `<intent-filter>`s by which the system offers the activity to view a file of one of
/// `associations`, as a `content:` or a `file:` URI: a type's MIME type, where it has one; its
/// extensions at the end of the URI's path, of any type; and, for a type the app defines, any file
/// of `application/octet-stream`, once for all of them.
///
/// Android has no way for an app to teach the system a new extension, so a file of a type the app
/// defines reaches it as `application/octet-stream`, and the Files app's Downloads view may name
/// a download by a number, with no extension to match: only the untyped filter offers the app for
/// it, which offers the app for every untyped file too.
fn file_intent_filters(associations: &[FileAssociation]) -> String {
    let mut filters = String::new();
    for association in associations {
        if let Some(mime_type) = &association.mime_type {
            filters += &view_filter(&format!(
                r#"<data android:mimeType="{}" />"#,
                xml_escape(mime_type)
            ));
        }
        filters += &view_filter(&extension_data(&association.extensions));
    }
    if associations.iter().any(|association| association.owner) {
        filters += &view_filter(r#"<data android:mimeType="application/octet-stream" />"#);
    }
    filters
}

/// The data of a filter that matches a path ending in one of `extensions`, of any type: a path is
/// matched only where the filter names a host.
fn extension_data(extensions: &[String]) -> String {
    let suffixes = extensions.iter().map(|extension| {
        format!(
            r#"<data android:pathSuffix=".{}" />"#,
            xml_escape(extension)
        )
    });
    [
        r#"<data android:host="*" />"#.to_owned(),
        r#"<data android:mimeType="*/*" />"#.to_owned(),
    ]
    .into_iter()
    .chain(suffixes)
    .collect::<Vec<_>>()
    .join("\n        ")
}

/// An `<intent-filter>` for viewing a `content:` or a `file:` URI that `data` matches further.
fn view_filter(data: &str) -> String {
    format!(
        r#"      <intent-filter>
        <action android:name="android.intent.action.VIEW" />
        <category android:name="android.intent.category.DEFAULT" />
        <data android:scheme="content" />
        <data android:scheme="file" />
        {data}
      </intent-filter>
"#
    )
}

/// One `<uses-permission>` for each of [`PERMISSIONS`], then for each of `the_apps` not among
/// them, once each.
fn uses_permissions(the_apps: &[String]) -> String {
    let mut named: Vec<&str> = PERMISSIONS.to_vec();
    for permission in the_apps {
        if !named.contains(&permission.as_str()) {
            named.push(permission);
        }
    }
    named
        .into_iter()
        .map(|permission| {
            format!(
                "  <uses-permission android:name=\"{}\" />\n",
                xml_escape(permission)
            )
        })
        .collect()
}

/// One `<uses-feature>` for each of `features`, not required: the app installs on a device without
/// it.
fn uses_features(features: &[&str]) -> String {
    features
        .iter()
        .map(|feature| {
            format!(
                "  <uses-feature android:name=\"{}\" android:required=\"false\" />\n",
                xml_escape(feature)
            )
        })
        .collect()
}

/// One `<intent>` for viewing a link of each of [`QUERIED_SCHEMES`].
fn queried_intents() -> String {
    QUERIED_SCHEMES
        .iter()
        .map(|scheme| {
            format!(
                r#"    <intent>
      <action android:name="android.intent.action.VIEW" />
      <data android:scheme="{scheme}" />
    </intent>
"#
            )
        })
        .collect()
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A `versionCode` that orders like the package version: `1.2.3` → 10203, with the minor
/// and patch numbers each given two digits.
pub(super) fn version_code(version: &str) -> u32 {
    let mut parts = version
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0).min(99);
    let patch = parts.next().unwrap_or(0).min(99);
    major
        .saturating_mul(10_000)
        .saturating_add(minor.saturating_mul(100))
        .saturating_add(patch)
        .max(1)
}

/// Waft's manifest with each library's merged into it, the first listed first.
pub(super) fn merge_manifests(wafts: &str, libraries: &[AndroidLibrary]) -> Result<String> {
    let manifests: Vec<LibraryManifest> = libraries
        .iter()
        .filter_map(|library| {
            library.manifest.as_ref().map(|manifest| LibraryManifest {
                library: &library.name,
                manifest: &manifest.element,
            })
        })
        .collect();
    manifest_merger::merge(wafts, &manifests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_code_orders_like_the_package_version() {
        assert_eq!(version_code("0.1.0"), 100);
        assert_eq!(version_code("1.2.3"), 10203);
        assert!(version_code("1.10.0") > version_code("1.9.9"));
        assert_eq!(version_code("bogus"), 1, "never zero");
    }

    #[test]
    fn manifest_names_waft_activity_and_its_library() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My <App>",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: true,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
            devices: &UsageDescriptions::default(),
        });
        assert!(manifest.contains(r#"package="com.example.my_app""#));
        assert!(manifest.contains(r#"android:name="rs.waft.WaftActivity""#));
        assert!(manifest.contains(r#"android:hasCode="true""#));
        assert!(manifest.contains(r#"android:value="my_app""#));
        assert!(manifest.contains(r#"android:label="My &lt;App&gt;""#));
        assert!(manifest.contains(r#"android:debuggable="true""#));
        assert!(!manifest.contains("ic_launcher"));
        assert!(manifest.contains(r#"android:targetSdkVersion="34""#));
        assert!(
            manifest.contains(r#"android:appCategory="productivity""#),
            "an uncategorised app is taken for a game and held at 60 Hz"
        );
        for change in [
            "orientation",
            "screenSize",
            "uiMode",
            "fontScale",
            "fontWeightAdjustment",
            "density",
        ] {
            assert!(
                manifest.contains(change),
                "the activity must absorb {change} rather than be recreated for it"
            );
        }
    }

    #[test]
    fn manifest_declares_a_document_activity_beside_the_launchers() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 1,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
            devices: &UsageDescriptions::default(),
        });
        let launcher =
            &manifest[manifest.find("<activity").unwrap()..manifest.find("</activity>").unwrap()];
        assert!(launcher.contains(r#"android:launchMode="singleTask""#));
        let document = &manifest[manifest
            .find(r#"android:name="rs.waft.WaftDocumentActivity""#)
            .expect("the document activity is declared")..];
        let document = &document[..document.find("</activity>").unwrap()];
        for attribute in [
            r#"android:exported="false""#,
            r#"android:launchMode="standard""#,
            r#"android:documentLaunchMode="intoExisting""#,
            r#"android:maxRecents="50""#,
            r#"android:value="my_app""#,
        ] {
            assert!(
                document.contains(attribute),
                "the document activity has {attribute}"
            );
        }
        assert!(document.contains(CONFIG_CHANGES));
        assert!(
            !document.contains("intent-filter"),
            "only the app starts it"
        );
    }

    #[test]
    fn manifest_declares_the_file_provider_and_the_queried_links() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
            devices: &UsageDescriptions::default(),
        });
        assert!(manifest.contains(r#"android:name="rs.waft.WaftFileProvider""#));
        assert!(manifest.contains(r#"android:authorities="com.example.my_app.waft.files""#));
        assert!(
            manifest.contains(r#"android:exported="false""#),
            "the provider shares only the URIs it grants"
        );
        assert!(manifest.contains(r#"android:grantUriPermissions="true""#));
        assert!(manifest.contains(r#"<data android:scheme="mailto" />"#));
        assert!(manifest.contains(
            r#"<meta-data android:name="rs.waft.queried_schemes" android:value="http,https,mailto" />"#
        ));
    }

    #[test]
    fn an_apps_own_permissions_follow_wafts_once_each() {
        let own = [
            "android.permission.INTERNET".to_owned(),
            "android.permission.POST_NOTIFICATIONS".to_owned(),
            "android.permission.INTERNET".to_owned(),
        ];
        let asked = uses_permissions(&own);
        assert_eq!(
            asked.matches("android.permission.INTERNET").count(),
            1,
            "{asked}"
        );
        assert_eq!(
            asked
                .matches("android.permission.POST_NOTIFICATIONS")
                .count(),
            1,
            "{asked}"
        );
        assert!(
            asked.find("POST_NOTIFICATIONS") < asked.find("INTERNET"),
            "Waft's own come first: {asked}"
        );
    }

    #[test]
    fn manifest_declares_the_notification_receiver_and_its_permissions() {
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &[],
            devices: &UsageDescriptions::default(),
        });
        assert!(manifest.contains(
            r#"<receiver android:name="rs.waft.WaftNotificationReceiver" android:exported="false" />"#
        ));
        assert!(manifest.contains(
            r#"<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />"#
        ));
        assert!(manifest.contains(
            r#"<uses-permission android:name="android.permission.SCHEDULE_EXACT_ALARM" />"#
        ));
        assert!(
            manifest.find("<uses-permission") < manifest.find("<application"),
            "permissions belong to the manifest, before the application"
        );
    }

    #[test]
    fn a_declared_camera_is_asked_for_and_its_feature_not_required() {
        let camera = UsageDescriptions {
            camera: Some("Takes photos.".into()),
            microphone: None,
        };
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &[],
            permissions: &["android.permission.CAMERA".to_owned()],
            devices: &camera,
        });
        assert_eq!(
            manifest
                .matches(r#"<uses-permission android:name="android.permission.CAMERA" />"#)
                .count(),
            1,
            "asked for once, though the app names it too"
        );
        assert!(manifest.contains(
            r#"<uses-feature android:name="android.hardware.camera" android:required="false" />"#
        ));
        assert!(manifest.find("<uses-feature") < manifest.find("<application"));
    }

    fn association(extensions: &[&str], mime_type: Option<&str>, owner: bool) -> FileAssociation {
        FileAssociation {
            extensions: extensions
                .iter()
                .map(|extension| extension.to_string())
                .collect(),
            name: "A document".into(),
            owner,
            role: Default::default(),
            identifier: None,
            mime_type: mime_type.map(str::to_owned),
        }
    }

    #[test]
    fn a_typed_association_is_viewed_by_its_mime_type_and_its_extensions() {
        let filters = file_intent_filters(&[association(
            &["md", "markdown"],
            Some("text/markdown"),
            false,
        )]);
        assert_eq!(filters.matches("<intent-filter>").count(), 2);
        assert!(filters.contains(r#"<action android:name="android.intent.action.VIEW" />"#));
        assert!(filters.contains(r#"<category android:name="android.intent.category.DEFAULT" />"#));
        assert!(filters.contains(r#"<data android:scheme="content" />"#));
        assert!(filters.contains(r#"<data android:scheme="file" />"#));
        assert!(filters.contains(r#"<data android:mimeType="text/markdown" />"#));
        assert!(filters.contains(r#"<data android:pathSuffix=".md" />"#));
        assert!(filters.contains(r#"<data android:pathSuffix=".markdown" />"#));
        assert!(
            filters.contains(r#"<data android:host="*" />"#),
            "a path is matched only where the filter names a host"
        );
        assert!(
            !filters.contains("application/octet-stream"),
            "a type the system knows needs no untyped filter"
        );
    }

    #[test]
    fn an_owned_type_is_also_offered_for_untyped_files_once() {
        let filters = file_intent_filters(&[
            association(&["wapk"], None, true),
            association(&["wapp"], None, true),
        ]);
        assert!(filters.contains(r#"<data android:pathSuffix=".wapk" />"#));
        assert!(filters.contains(r#"<data android:pathSuffix=".wapp" />"#));
        assert_eq!(
            filters
                .matches(r#"<data android:mimeType="application/octet-stream" />"#)
                .count(),
            1
        );
    }

    #[test]
    fn the_activity_carries_the_file_filters() {
        let associations = [association(&["wapk"], None, true)];
        let manifest = manifest(&Manifest {
            package: "com.example.my_app",
            label: "My App",
            library: "my_app",
            version_code: 100,
            version_name: "0.1.0",
            min_sdk: 31,
            target_sdk: 34,
            debuggable: false,
            has_icon: false,
            category: "productivity",
            file_associations: &associations,
            permissions: &[],
            devices: &UsageDescriptions::default(),
        });
        let activity =
            &manifest[manifest.find("<activity").unwrap()..manifest.find("</activity>").unwrap()];
        assert!(activity.contains(r#"<data android:pathSuffix=".wapk" />"#));
        assert!(activity.contains("application/octet-stream"));
    }
}
