//! A notification's image as the file a toast shows it from: a PNG in the temporary folder, named
//! by a hash of its pixels, so posting the same image again reuses its file rather than adding
//! one. The file stays: the system reads it whenever it draws the toast, in the Action Center too.

use std::fmt::Write;
use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use inset_embedder::IconImage;

use super::toast::fnv1a;

/// The `file:` URI of `image`'s file, written if it is not there yet.
pub(crate) fn image_uri(image: &IconImage) -> io::Result<String> {
    let path = image_path(image)?;
    if !path.exists() {
        write_png(image, &path)?;
    }
    Ok(file_uri(&path))
}

/// Where `image`'s file is: named by its size and pixels.
fn image_path(image: &IconImage) -> io::Result<PathBuf> {
    let folder = std::env::temp_dir().join("inset-notifications");
    fs::create_dir_all(&folder)?;
    let mut named = Vec::with_capacity(8 + image.rgba.len());
    named.extend(image.width.to_le_bytes());
    named.extend(image.height.to_le_bytes());
    named.extend_from_slice(&image.rgba);
    Ok(folder.join(format!("{:016x}.png", fnv1a(&named))))
}

fn write_png(image: &IconImage, path: &Path) -> io::Result<()> {
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        image.width,
        image.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&image.rgba)?;
    Ok(())
}

/// `path`, absolute, as a `file:` URI: `file:///C:/Users/…`, each byte of the path's UTF-8 but a
/// letter, a digit, a separator or `-._~:` percent-encoded.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file:///");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'\\' | b'/' => uri.push('/'),
            byte if byte.is_ascii_alphanumeric() || b"-._~:".contains(&byte) => {
                uri.push(char::from(byte));
            }
            byte => {
                let _ = write!(uri, "%{byte:02X}");
            }
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn an_image_is_written_once_and_its_file_reused() {
        let image = IconImage {
            width: 1,
            height: 1,
            rgba: Arc::from([255, 0, 0, 255]),
            template: false,
        };
        let uri = image_uri(&image).expect("written");
        assert!(uri.starts_with("file:///") && uri.ends_with(".png"));
        assert_eq!(image_uri(&image).expect("found"), uri);
        let path = image_path(&image).expect("its path");
        let decoder = png::Decoder::new(io::BufReader::new(File::open(path).expect("its file")));
        let header = decoder.read_info().expect("a PNG").info().clone();
        assert_eq!((header.width, header.height), (1, 1));
    }

    #[test]
    fn a_path_is_a_file_uri_with_its_odd_bytes_encoded() {
        assert_eq!(
            file_uri(Path::new(
                r"C:\Users\José Doe\AppData\Local\Temp\inset-notifications\a.png"
            )),
            "file:///C:/Users/Jos%C3%A9%20Doe/AppData/Local/Temp/inset-notifications/a.png"
        );
    }
}
