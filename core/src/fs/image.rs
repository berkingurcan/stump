use image::{imageops, io::Reader, DynamicImage, EncodableLayout, GenericImageView};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
	fs::File,
	io::{Read, Write},
	path::{Path, PathBuf},
};
use webp::{Encoder, WebPMemory};

use crate::{
	config::get_thumbnails_dir,
	prisma::media,
	types::{alias::ProcessFileResult, errors::ProcessFileError},
};

use super::media_file;

pub fn get_image_bytes<P: AsRef<Path>>(path: P) -> ProcessFileResult<Vec<u8>> {
	let mut file = File::open(path)?;

	let mut buf = Vec::new();
	file.read_to_end(&mut buf)?;

	Ok(buf)
}

pub fn webp_from_path<P: AsRef<Path>>(file_path: P) -> ProcessFileResult<Vec<u8>> {
	let image = Reader::open(file_path.as_ref())?
		.with_guessed_format()?
		.decode()?;

	let (width, height) = image.dimensions();

	// TODO: perhaps make size factor configurable?
	let size_factor = 0.5;

	let image = DynamicImage::ImageRgba8(imageops::resize(
		&image,
		(width as f64 * size_factor) as u32,
		(height as f64 * size_factor) as u32,
		// TODO: determine best filter
		imageops::FilterType::Triangle,
	));

	let encoder: Encoder = Encoder::from_image(&image)
		.map_err(|err| ProcessFileError::WebpEncodeError(err.to_string()))?;

	let encoded_webp: WebPMemory = encoder.encode(65f32);

	Ok(encoded_webp.as_bytes().to_vec())
}

// TODO: this is **super** slow!!!!
pub fn webp_from_bytes(bytes: &[u8]) -> ProcessFileResult<Vec<u8>> {
	let image = image::load_from_memory(bytes)?;

	let (width, height) = image.dimensions();

	// TODO: perhaps make size factor configurable?
	let size_factor = 0.5;

	let image = DynamicImage::ImageRgba8(imageops::resize(
		&image,
		(width as f64 * size_factor) as u32,
		(height as f64 * size_factor) as u32,
		// TODO: determine best filter
		imageops::FilterType::Triangle,
	));

	let encoder: Encoder = Encoder::from_image(&image)
		.map_err(|err| ProcessFileError::WebpEncodeError(err.to_string()))?;

	let encoded_webp: WebPMemory = encoder.encode(5.0);

	Ok(encoded_webp.as_bytes().to_vec())
}

pub fn generate_thumbnail(id: &str, path: &str) -> ProcessFileResult<PathBuf> {
	let (_, buf) = media_file::get_page(path, 1)?;
	let webp_buf = webp_from_bytes(&buf)?;

	let thumbnail_path = get_thumbnails_dir().join(format!("{}.webp", &id));

	if !thumbnail_path.exists() {
		let mut webp_image = File::create(&thumbnail_path)?;

		webp_image.write_all(&webp_buf)?;
	} else {
		log::trace!("Thumbnail already exists for {}", &id);
	}

	Ok(thumbnail_path)
}

// TODO: does this need to return a result?
pub fn generate_thumbnails(media: Vec<media::Data>) -> ProcessFileResult<Vec<PathBuf>> {
	log::debug!("Enter generate_thumbnails");

	// TODO: this might make the stack overflow lol
	let results = media
		.into_par_iter()
		// .with_max_len(5)
		.map(|m| generate_thumbnail(m.id.as_str(), &m.path.as_str()))
		.filter_map(|res| {
			if res.is_err() {
				log::error!("Error generating thumbnail: {:?}", res.err());
				None
			} else {
				res.ok()
			}
		})
		.collect::<Vec<PathBuf>>();

	log::debug!("Generated the following thumbnails: {:?}", results);

	Ok(results)
}

pub fn get_thumbnail_path(id: &str) -> Option<PathBuf> {
	let thumbnail_path = get_thumbnails_dir().join(format!("{}.webp", id));

	if thumbnail_path.exists() {
		Some(thumbnail_path)
	} else {
		None
	}
}

pub fn remove_thumbnail(id: &str) -> ProcessFileResult<()> {
	let thumbnail_path = get_thumbnails_dir().join(format!("{}.webp", id));

	if thumbnail_path.exists() {
		std::fs::remove_file(thumbnail_path)?;
	}

	Ok(())
}

pub fn remove_thumbnails(id_list: &[String]) -> ProcessFileResult<()> {
	for id in id_list {
		// TODO: not sure I want the entire process to fail if one thumbnail fails to delete...
		// for now, I will leave it as is. I can't see to many cases where this would happen, but
		// it's obviously possible.
		remove_thumbnail(id)?;
	}

	Ok(())
}

// TODO: tests!
#[cfg(test)]
mod tests {}
