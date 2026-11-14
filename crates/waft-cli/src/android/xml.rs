//! An Android manifest as a tree of elements, read from text and written back: what the manifest
//! merger works on.
//!
//! Attribute names carry their namespace as the prefix Android's tools write: `android:` for
//! Android's attributes and `tools:` for the build tools', whatever prefix a file declares for
//! either; an attribute of any other namespace is refused, since nothing here knows what it means.
//! A manifest holds no text, only whitespace between elements, so text is refused too. A later
//! editor adding text or another namespace must teach [`write`] to write it back.

use anyhow::{Result, bail};

/// Android's namespace, `android:`.
pub const ANDROID_NAMESPACE: &str = "http://schemas.android.com/apk/res/android";
/// The build tools' namespace, `tools:`, which the manifest merger reads and the package never
/// carries.
pub const TOOLS_NAMESPACE: &str = "http://schemas.android.com/tools";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// `android:name`, `tools:ignore`, or a bare `package`.
    pub name: String,
    pub value: String,
}

impl Element {
    pub fn new(name: &str) -> Element {
        Element {
            name: name.to_owned(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }
}

/// The root element of the XML `text`.
pub fn parse(text: &str) -> Result<Element> {
    let document = roxmltree::Document::parse(text)?;
    element(document.root_element())
}

fn element(node: roxmltree::Node) -> Result<Element> {
    let name = node.tag_name();
    if name.namespace().is_some() {
        bail!("<{}> is in a namespace", name.name());
    }
    let mut attributes = Vec::new();
    for attribute in node.attributes() {
        attributes.push(Attribute {
            name: qualified_name(name.name(), attribute.namespace(), attribute.name())?,
            value: attribute.value().to_owned(),
        });
    }
    let mut children = Vec::new();
    for child in node.children() {
        if child.is_element() {
            children.push(element(child)?);
        } else if child.is_text() && !child.text().unwrap_or_default().trim().is_empty() {
            bail!("<{}> holds text", name.name());
        }
    }
    Ok(Element {
        name: name.name().to_owned(),
        attributes,
        children,
    })
}

/// `name` with the prefix of its `namespace`, of an attribute of `<element>`.
fn qualified_name(element: &str, namespace: Option<&str>, name: &str) -> Result<String> {
    Ok(match namespace {
        None => name.to_owned(),
        Some(ANDROID_NAMESPACE) => format!("android:{name}"),
        Some(TOOLS_NAMESPACE) => format!("tools:{name}"),
        Some(other) => bail!("<{element}> has the attribute {name} of the namespace {other}"),
    })
}

/// `root` as a manifest's text, declaring Android's namespace.
pub fn write(root: &Element) -> String {
    let mut text = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    write_element(root, 0, &mut text);
    text
}

fn write_element(element: &Element, depth: usize, text: &mut String) {
    let indent = "  ".repeat(depth);
    text.push_str(&indent);
    text.push('<');
    text.push_str(&element.name);
    if depth == 0 {
        text.push_str(&format!(" xmlns:android=\"{ANDROID_NAMESPACE}\""));
    }
    for attribute in &element.attributes {
        text.push_str(&format!(
            "\n{indent}    {}=\"{}\"",
            attribute.name,
            escape(&attribute.value)
        ));
    }
    if element.children.is_empty() {
        text.push_str(" />\n");
        return;
    }
    text.push_str(">\n");
    for child in &element.children {
        write_element(child, depth + 1, text);
    }
    text.push_str(&format!("{indent}</{}>\n", element.name));
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_reads_back_as_it_was_written() {
        let text = r#"<manifest xmlns:a="http://schemas.android.com/apk/res/android" package="p">
  <application a:label="A &amp; B"><meta-data a:name="k" a:value="v"/></application>
</manifest>"#;
        let root = parse(text).unwrap();
        assert_eq!(root.attribute("package"), Some("p"));
        let application = &root.children[0];
        assert_eq!(
            application.attribute("android:label"),
            Some("A & B"),
            "any prefix of Android's namespace is `android:`"
        );
        assert_eq!(parse(&write(&root)).unwrap(), root);
        assert!(write(&root).contains(r#"android:label="A &amp; B""#));
    }

    #[test]
    fn an_unknown_namespace_or_text_is_refused() {
        let foreign = r#"<manifest xmlns:x="urn:x"><application x:y="1"/></manifest>"#;
        assert!(parse(foreign).unwrap_err().to_string().contains("urn:x"));
        assert!(parse("<manifest>words</manifest>").is_err());
    }
}
