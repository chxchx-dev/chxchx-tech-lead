//! Images for a chat turn: whatever the person drops, picks or pastes is reduced to what a
//! model can actually use, so the agent gets the content without paying for pixels it cannot see.
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::path::{Path, PathBuf};

/// Longest side agents make use of; bigger images are shrunk by the provider anyway.
pub const MAX_SIDE: u32 = 1568;
/// Images below this size are sent untouched (no recompression artifacts).
const KEEP_BYTES: u64 = 1_500_000;
const MAX_INPUT_BYTES: u64 = 20 * 1024 * 1024;
const THUMB_SIDE: u32 = 120;
const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

#[derive(Clone, Debug)]
pub struct Prepared {
    pub path: PathBuf,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub original_bytes: u64,
    pub bytes: u64,
    pub thumb_width: u32,
    pub thumb_height: u32,
    pub thumb_rgba: Vec<u8>,
}

impl Prepared {
    /// Rough size the model reads it as, in tokens.
    pub fn tokens(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height) / 750
    }

    pub fn saved(&self) -> bool {
        self.bytes + 1024 < self.original_bytes
    }
}

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

fn staging() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("chxchx-studio-images");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("No pude preparar la imagen: {error}"))?;
    Ok(dir)
}

fn stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

fn has_transparency(image: &DynamicImage) -> bool {
    image.color().has_alpha() && image.to_rgba8().pixels().any(|p| p.0[3] < 250)
}

/// Reads, shrinks (longest side `MAX_SIDE`) and re-encodes an image from disk.
pub fn prepare(source: &Path) -> Result<Prepared, String> {
    if !is_image(source) {
        return Err("Solo se pueden adjuntar PNG, JPG, GIF o WebP.".to_owned());
    }
    let original = std::fs::metadata(source)
        .map_err(|_| format!("No encuentro {}", source.display()))?
        .len();
    if original > MAX_INPUT_BYTES {
        return Err("La imagen supera 20 MB.".to_owned());
    }
    let image = image::open(source).map_err(|error| format!("No pude leer la imagen: {error}"))?;
    let name = source
        .file_stem()
        .map_or_else(|| "imagen".to_owned(), |s| s.to_string_lossy().into_owned());
    finish(image, &name, original, Some(source))
}

fn finish(
    image: DynamicImage,
    name: &str,
    original: u64,
    source: Option<&Path>,
) -> Result<Prepared, String> {
    let (width, height) = image.dimensions();
    let oversized = width.max(height) > MAX_SIDE;
    let ext = source
        .and_then(|s| s.extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let plain_format = matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg"));
    let dir = staging()?;
    let untouched = source.is_some() && !oversized && plain_format && original <= KEEP_BYTES;
    let (path, bytes, out) = if let (true, Some(source)) = (untouched, source) {
        // Small and already in a format every agent reads: send the file as it is.
        let target = dir.join(format!(
            "{}-{name}.{}",
            stamp(),
            ext.as_deref().unwrap_or("png")
        ));
        std::fs::copy(source, &target).map_err(|error| error.to_string())?;
        (target, original, image)
    } else {
        let image = if oversized {
            image.resize(MAX_SIDE, MAX_SIDE, FilterType::Triangle)
        } else {
            image
        };
        let alpha = has_transparency(&image);
        let (format, extension) = if alpha || original <= KEEP_BYTES {
            (ImageFormat::Png, "png")
        } else {
            (ImageFormat::Jpeg, "jpg")
        };
        let target = dir.join(format!("{}-{name}.{extension}", stamp()));
        let mut buffer = std::io::Cursor::new(Vec::new());
        match format {
            ImageFormat::Jpeg => DynamicImage::ImageRgb8(image.to_rgb8()).write_with_encoder(
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 85),
            ),
            _ => image.write_to(&mut buffer, format),
        }
        .map_err(|error| format!("No pude preparar la imagen: {error}"))?;
        let data = buffer.into_inner();
        std::fs::write(&target, &data).map_err(|error| error.to_string())?;
        (target, data.len() as u64, image)
    };
    let (width, height) = out.dimensions();
    let thumb = out.thumbnail(THUMB_SIDE, THUMB_SIDE).to_rgba8();
    Ok(Prepared {
        path,
        name: name.to_owned(),
        width,
        height,
        original_bytes: original.max(bytes),
        bytes,
        thumb_width: thumb.width(),
        thumb_height: thumb.height(),
        thumb_rgba: thumb.into_raw(),
    })
}

/// An image on the clipboard (a screenshot, usually), if there is one.
pub fn from_clipboard() -> Result<Option<Prepared>, String> {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return Ok(None);
    };
    let Ok(data) = clipboard.get_image() else {
        return Ok(None);
    };
    let (width, height) = (data.width as u32, data.height as u32);
    let Some(buffer) = image::RgbaImage::from_raw(width, height, data.bytes.into_owned()) else {
        return Ok(None);
    };
    let raw = u64::from(width) * u64::from(height) * 4;
    finish(DynamicImage::ImageRgba8(buffer), "captura", raw, None).map(Some)
}

/// Removes a prepared copy; the originals are never touched.
pub fn discard(prepared: &Prepared) {
    if prepared
        .path
        .starts_with(std::env::temp_dir().join("chxchx-studio-images"))
    {
        let _ = std::fs::remove_file(&prepared.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-images-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn noise(width: u32, height: u32) -> DynamicImage {
        let mut state = 7u32;
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(width, height, |_, _| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            image::Rgb([(state >> 8) as u8, (state >> 16) as u8, (state >> 24) as u8])
        }))
    }

    #[test]
    fn a_huge_photo_is_shrunk_and_recompressed() {
        let dir = scratch("big");
        let file = dir.join("foto.png");
        noise(3200, 2000).save(&file).unwrap();
        let prepared = prepare(&file).unwrap();
        assert_eq!(prepared.width.max(prepared.height), MAX_SIDE);
        assert!(
            prepared.path.extension().is_some_and(|e| e == "jpg"),
            "{:?}",
            prepared.path
        );
        assert!(prepared.bytes < prepared.original_bytes && prepared.saved());
        assert!(prepared.tokens() > 0 && prepared.thumb_width <= 120);
        assert!(file.exists(), "the original is left alone");
        discard(&prepared);
        assert!(!prepared.path.exists());
        // The photo is 18 MB of noise: do not leave it in the temporary folder run after run.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_small_image_travels_untouched() {
        let dir = scratch("small");
        let file = dir.join("icono.png");
        noise(64, 64).save(&file).unwrap();
        let prepared = prepare(&file).unwrap();
        assert_eq!((prepared.width, prepared.height), (64, 64));
        assert_eq!(
            std::fs::read(&prepared.path).unwrap(),
            std::fs::read(&file).unwrap()
        );
        discard(&prepared);
    }

    #[test]
    fn what_is_not_a_readable_image_is_refused_with_a_reason() {
        let dir = scratch("bad");
        let text = dir.join("notas.txt");
        std::fs::write(&text, "x").unwrap();
        assert!(prepare(&text).unwrap_err().contains("PNG"));
        let broken = dir.join("roto.png");
        std::fs::write(&broken, "no soy un png").unwrap();
        assert!(prepare(&broken).unwrap_err().contains("leer"));
        assert!(prepare(&dir.join("falta.png")).is_err());
        assert!(is_image(Path::new("A.JPEG")) && !is_image(Path::new("a.svg")));
    }
}
