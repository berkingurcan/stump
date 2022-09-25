use rocket::http::ContentType;
use std::{path::Path, str::FromStr};

use crate::types::{
	errors::ProcessFileError,
	models::{
		library::LibraryOptions,
		media::{MediaMetadata, ProcessedMediaFile},
	},
};

use super::{
	epub::{get_epub_cover, process_epub},
	rar::{get_rar_image, process_rar},
	zip::{get_zip_image, process_zip},
};

// FIXME: this module does way too much. It should be cleaned up, way too many vaguely
// similar things shoved in here with little distinction.

// TODO: replace all these match statements with an custom enum that handles it all.
// The enum itself will have some repetition, however it'll be cleaner than
// doing this stuff over and over as this file currently does.

// TODO: move trait, maybe merge with another.
pub trait IsImage {
	fn is_image(&self) -> bool;
}

pub fn process_comic_info(buffer: String) -> Option<MediaMetadata> {
	if buffer.is_empty() {
		return None;
	}

	match serde_xml_rs::from_str(&buffer) {
		Ok(info) => Some(info),
		_ => None,
	}
}

// I am adding the required and currently missing types I need to Rocket
// (https://github.com/SergioBenitez/Rocket/pull/2221), but in the meantime
// need to use this for now when encountering missing mimes. These are
// almost correct replacements, e.g. opf is supposed to be application/oebps-package+xml,
// not just application/xml.
fn temporary_content_workarounds(extension: &str) -> ContentType {
	if extension == "opf" || extension == "ncx" {
		return ContentType::XML;
	}

	ContentType::Any
}

pub fn guess_content_type(file: &str) -> ContentType {
	let file = Path::new(file);

	let extension = file.extension().unwrap_or_default();

	let extension = extension.to_string_lossy().to_string();

	// TODO: if this fails manually check the extension
	match ContentType::from_extension(&extension) {
		Some(content_type) => content_type,
		// None => ContentType::Any,
		None => temporary_content_workarounds(&extension),
	}
}

// FIXME: replace some of these once Rocket PR is merged
pub fn get_content_type_from_mime(mime: &str) -> ContentType {
	ContentType::from_str(mime).unwrap_or(match mime {
		"application/pdf" => ContentType::PDF,
		// "application/epub+zip" => ContentType::EPUB,
		"application/zip" => ContentType::ZIP,
		"application/vnd.comicbook+zip" => ContentType::ZIP,
		// "application/vnd.rar" => ContentType::RAR,
		// "application/vnd.comicbook-rar" => ContentType::RAR,
		"image/png" => ContentType::PNG,
		"image/jpeg" => ContentType::JPEG,
		"image/webp" => ContentType::WEBP,
		"image/gif" => ContentType::GIF,
		// FIXME: replace one PR is merged (AGH, will it ever get merged??)
		"application/xhtml+xml" => ContentType::XML,
		_ => ContentType::Any,
	})
}

/// Guess the MIME type of a file based on its extension.
pub fn guess_mime(path: &Path) -> Option<String> {
	let extension = path.extension().and_then(|ext| ext.to_str());

	if extension.is_none() {
		log::warn!(
			"Unable to guess mime for file without extension: {:?}",
			path
		);
		return None;
	}

	let extension = extension.unwrap();

	let content_type = ContentType::from_extension(extension);

	if let Some(content_type) = content_type {
		return Some(content_type.to_string());
	}

	// TODO: add more?
	match extension.to_lowercase().as_str() {
		"pdf" => Some("application/pdf".to_string()),
		"epub" => Some("application/epub+zip".to_string()),
		"zip" => Some("application/zip".to_string()),
		"cbz" => Some("application/vnd.comicbook+zip".to_string()),
		"rar" => Some("application/vnd.rar".to_string()),
		"cbr" => Some("application/vnd.comicbook-rar".to_string()),
		"png" => Some("image/png".to_string()),
		"jpg" => Some("image/jpeg".to_string()),
		"jpeg" => Some("image/jpeg".to_string()),
		"webp" => Some("image/webp".to_string()),
		"gif" => Some("image/gif".to_string()),
		_ => None,
	}
}

/// Infer the MIME type of a file. If the MIME type cannot be inferred via reading
/// the first few bytes of the file, then the file extension is used via `guess_mime`.
pub fn infer_mime_from_path(path: &Path) -> Option<String> {
	match infer::get_from_path(path) {
		Ok(mime) => {
			log::debug!("Inferred mime for file {:?}: {:?}", path, mime);
			mime.and_then(|m| Some(m.mime_type().to_string()))
		},
		Err(e) => {
			log::warn!(
				"Unable to infer mime for file {:?}: {:?}",
				path,
				e.to_string()
			);

			guess_mime(path)
		},
	}
}

pub fn get_page(
	file: &str,
	page: i32,
) -> Result<(ContentType, Vec<u8>), ProcessFileError> {
	let mime = guess_mime(Path::new(file));

	match mime.as_deref() {
		Some("application/zip") => get_zip_image(file, page),
		Some("application/vnd.comicbook+zip") => get_zip_image(file, page),
		Some("application/vnd.rar") => get_rar_image(file, page),
		Some("application/vnd.comicbook-rar") => get_rar_image(file, page),
		Some("application/epub+zip") => {
			if page == 1 {
				get_epub_cover(file)
			} else {
				Err(ProcessFileError::UnsupportedFileType(format!(
					"You may only request the cover page (first page) for epub files on this endpoint"
				)))
			}
		},
		None => Err(ProcessFileError::Unknown(format!(
			"Unable to determine mime type for file: {:?}",
			file
		))),
		_ => Err(ProcessFileError::UnsupportedFileType(file.to_string())),
	}
}

pub fn process(
	path: &Path,
	options: &LibraryOptions,
) -> Result<ProcessedMediaFile, ProcessFileError> {
	log::debug!("Processing entry {:?} with options: {:?}", path, options);

	let mime = infer_mime_from_path(path);

	match mime.as_deref() {
		Some("application/zip") => process_zip(path),
		Some("application/vnd.comicbook+zip") => process_zip(path),
		Some("application/vnd.rar") => process_rar(path, options),
		Some("application/vnd.comicbook-rar") => process_rar(path, options),
		Some("application/epub+zip") => process_epub(path),
		None => Err(ProcessFileError::Unknown(format!(
			"Unable to determine mime type for file: {:?}",
			path
		))),
		_ => Err(ProcessFileError::UnsupportedFileType(
			path.to_string_lossy().into_owned(),
		)),
	}
}
