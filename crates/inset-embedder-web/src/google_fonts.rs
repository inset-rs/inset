//! A Google Fonts CSS2 response, read as one family's unicode-range slices.
//!
//! Flutter counterpart: the slice table in `font_fallback_data.dart`, which
//! `dev/roll_fallback_fonts.dart` generates from these same responses. Here
//! the response is read at run time, once per family, on the first character
//! that needs it.

const CSS2: &str = "https://fonts.googleapis.com/css2";

/// One `@font-face` block: the characters a file carries, and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    pub ranges: Vec<(u32, u32)>,
    pub url: String,
}

impl Slice {
    pub fn covers(&self, code: u32) -> bool {
        self.ranges
            .iter()
            .any(|(start, end)| (*start..=*end).contains(&code))
    }
}

/// The CSS2 request for a family.
///
/// `variable` asks for the whole weight axis, which Google answers with
/// variable files; a family without that axis answers 400 Bad Request, so
/// only the family whose weights the interface uses asks for it.
pub fn css2_url(family: &str, variable: bool) -> String {
    let axis = if variable { ":wght@100..900" } else { "" };
    format!(
        "{CSS2}?family={}{axis}&display=block",
        family.replace(' ', "+")
    )
}

/// Every `@font-face` in a response that declares a `unicode-range`. A block
/// without one says nothing about which characters its file carries.
pub fn parse_slices(css: &str) -> Vec<Slice> {
    css.split("@font-face")
        .skip(1)
        .filter_map(|block| {
            let ranges = parse_ranges(declaration(block, "unicode-range")?);
            let url = font_url(declaration(block, "src")?)?;
            (!ranges.is_empty()).then_some(Slice { ranges, url })
        })
        .collect()
}

/// The value of `name:` in one block, up to its `;`.
fn declaration<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let at = block.find(name)? + name.len();
    let rest = block[at..].trim_start().strip_prefix(':')?;
    let end = rest.find(';')?;
    Some(rest[..end].trim())
}

/// `U+0000-00FF, U+0131, U+1F1E6-1F1FF` → inclusive pairs.
fn parse_ranges(value: &str) -> Vec<(u32, u32)> {
    value
        .split(',')
        .filter_map(|part| {
            let hex = part
                .trim()
                .strip_prefix("U+")
                .or_else(|| part.trim().strip_prefix("u+"))?;
            let (start, end) = hex.split_once('-').unwrap_or((hex, hex));
            Some((
                u32::from_str_radix(start, 16).ok()?,
                u32::from_str_radix(end, 16).ok()?,
            ))
        })
        .collect()
}

fn font_url(src: &str) -> Option<String> {
    let rest = src.split("url(").nth(1)?;
    let rest = rest.trim_start_matches(['\'', '"']);
    let end = rest.find(['\'', '"', ')'])?;
    let url = rest[..end].trim();
    (!url.is_empty()).then(|| url.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"
/* greek */
@font-face {
  font-family: 'Roboto';
  font-style: normal;
  font-weight: 100 900;
  src: url(https://fonts.gstatic.com/s/roboto/v51/greek.woff2) format('woff2');
  unicode-range: U+0370-0377, U+037A-037F, U+0384-038A, U+038C, U+038E-03A1, U+03A3-03FF;
}
/* latin */
@font-face {
  font-family: 'Roboto';
  font-style: normal;
  font-weight: 100 900;
  src: url(https://fonts.gstatic.com/s/roboto/v51/latin.woff2) format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+2000-206F;
}
"#;

    #[test]
    fn a_response_becomes_slices_in_order() {
        let slices = parse_slices(RESPONSE);
        assert_eq!(slices.len(), 2);
        assert_eq!(
            slices[0].url,
            "https://fonts.gstatic.com/s/roboto/v51/greek.woff2"
        );
        assert_eq!(slices[0].ranges[0], (0x0370, 0x0377));
        assert_eq!(slices[0].ranges[3], (0x038C, 0x038C));
        assert_eq!(
            slices[1].ranges,
            vec![(0x0000, 0x00FF), (0x0131, 0x0131), (0x2000, 0x206F)]
        );
    }

    #[test]
    fn a_slice_covers_its_ranges_inclusively() {
        let slices = parse_slices(RESPONSE);
        assert!(slices[0].covers(0x03BB));
        assert!(slices[0].covers(0x03FF));
        assert!(!slices[0].covers(0x0400));
        assert!(!slices[0].covers(0x0041));
        assert!(slices[1].covers(0x2026));
    }

    #[test]
    fn a_block_without_a_range_is_dropped() {
        let css = "@font-face { font-family: 'X'; src: url(https://a/b.woff2); }";
        assert!(parse_slices(css).is_empty());
    }

    #[test]
    fn only_the_variable_request_asks_for_the_axis() {
        assert_eq!(
            css2_url("Roboto", true),
            "https://fonts.googleapis.com/css2?family=Roboto:wght@100..900&display=block"
        );
        assert_eq!(
            css2_url("Noto Sans Hebrew", false),
            "https://fonts.googleapis.com/css2?family=Noto+Sans+Hebrew&display=block"
        );
    }
}
