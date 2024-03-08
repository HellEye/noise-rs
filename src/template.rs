use std::{
	path::{Path, PathBuf},
	process::Command,
};

use crate::Args;
use image::{ImageBuffer, Luma, Pixel};
use rayon::prelude::*;
pub struct Template {
	pub size: (u32, u32),
	pub image_paths: Vec<String>,
	tmp_path: String,
}
#[derive(Debug)]
pub enum TemplateCreateError {
	Input(String),
	Unknown(String),
}
impl From<std::io::Error> for TemplateCreateError {
	fn from(e: std::io::Error) -> Self {
		Self::Unknown(e.to_string())
	}
}
trait Clamp {
	fn clamp(self, min: f32, max: f32) -> f32;
}

impl Clamp for f32 {
	fn clamp(self, min: f32, max: f32) -> f32 {
		if self < min {
			min
		} else if self > max {
			max
		} else {
			self
		}
	}
}

pub const TMP_DIR_NAME: &str = "noise";
pub const TEMPLATE_FILE_NAME: &str = "template";
pub const TEMPLATE_FILE_EXTENSION: &str = "png";
pub fn round_to_even(num: u32) -> u32 {
	if num % 2 == 0 {
		num
	} else {
		num - 1
	}
}
impl Template {
	pub fn get_tmp_dir() -> PathBuf {
		let global_tmp_dir = std::env::temp_dir();
		global_tmp_dir.join(TMP_DIR_NAME)
	}
	pub fn get_ffmpeg_name() -> String {
		let tmp_dir = Template::get_tmp_dir();
		tmp_dir
			.join(format!(
				"{}%04d.{}",
				TEMPLATE_FILE_NAME, TEMPLATE_FILE_EXTENSION
			))
			.to_str()
			.expect("Could not convert path to string")
			.to_string()
	}
	pub fn new(args: &Args) -> Result<Self, TemplateCreateError> {
		// Input file path
		let current_dir = std::env::current_dir()?;
		let input_file_path = match current_dir.join(&args.input).to_str() {
			Some(input) => input.to_owned(),
			None => {
				return Err(TemplateCreateError::Input(
					"Could not convert path to string".into(),
				))
			}
		};
		let file = Path::new(input_file_path.as_str());
		if !file.exists() {
			return Err(TemplateCreateError::Input(format!(
				"File {} does not exist",
				input_file_path
			)));
		}
		// Temp template directory
		let tmp_dir = Template::get_tmp_dir();
		if !tmp_dir.exists() {
			std::fs::create_dir_all(&tmp_dir)?;
		}
		let template_file = Template::get_ffmpeg_name();
		// Run ffmpeg
		// Will split input file into frames
		let split_res = Command::new("ffmpeg")
			.args([
				"-i",
				input_file_path.as_str(),
				"-vf",
				"pad=ceil(iw/2)*2:ceil(ih/2)*2",
				template_file.as_str(),
			])
			.output()
			.expect("Could not run ffmpeg");
		println!(
			"ffmpeg output: {}\nstderr: \n{}",
			String::from_utf8_lossy(&split_res.stdout),
			String::from_utf8_lossy(&split_res.stderr)
		);
		// collect frames
		let mut image_paths = glob::glob(
			tmp_dir
				.join(format!(
					"{}*.{}",
					TEMPLATE_FILE_NAME, TEMPLATE_FILE_EXTENSION
				))
				.to_str()
				.unwrap(),
		)
		.expect("Bad glob")
		.map(|x| x.unwrap().to_str().unwrap().to_string())
		.collect::<Vec<String>>();
		image_paths.sort();

		// Load one image to get size
		let size = image::open(&image_paths[0])
			.expect("Could not load input image frame dimensions")
			.to_luma8()
			.dimensions();
		let size = (
			round_to_even(size.0 * args.upscale as u32),
			round_to_even(size.1 * args.upscale as u32),
		);
		Ok(Self {
			image_paths,
			tmp_path: tmp_dir.to_str().unwrap().to_string(),
			size,
		})
	}

	pub fn expand_black_by(&self, amount_px: u32) {
		// Load each image and expand black regions (above 127) by amount_px on each side, then save
		self.image_paths.par_iter().for_each(|image_path| {
			let image = image::open(image_path)
				.expect("Could not load image")
				.to_luma8();

			let values = {
				image.enumerate_pixels().map(|(x, y, _px)| {
					let px_value = get_pixel_wide(&image, x, y, amount_px, self.size);
					(x, y, px_value)
				})
			};
			let mut cloned_image = image.clone();

			values.for_each(|(x, y, template_px)| {
				if template_px < 20 {
					cloned_image.get_pixel_mut(x, y).invert();
				}
			});

			// Save the modified image
			cloned_image
				.save(image_path)
				.expect("Could not save image while expanding");
		});
	}
}

impl Drop for Template {
	fn drop(&mut self) {
		std::fs::remove_dir_all(&self.tmp_path).expect("Could not remove temp directory");
		// print!(
		// 	"Would remove:\n{}",
		// 	std::fs::read_dir(&self.tmp_path)
		// 		.unwrap()
		// 		.map(|x| x.unwrap().path().to_str().unwrap().to_string())
		// 		.collect::<Vec<String>>()
		// 		.join("\n")
		// );
	}
}

pub fn get_pixel_wide(
	image: &ImageBuffer<Luma<u8>, Vec<u8>>,
	x: u32,
	y: u32,
	window: u32,
	dimensions: (u32, u32),
) -> u8 {
	let half_window = if window % 2 == 0 {
		window / 2
	} else {
		(window - 1) / 2
	};
	let x_start = (x - half_window).clamp(0, dimensions.0 - 1);
	let x_end = (x + half_window).clamp(0, dimensions.0 - 1);
	let y_start = (y - half_window).clamp(0, dimensions.1 - 1);
	let y_end = (y + half_window).clamp(0, dimensions.1 - 1);
	for x in x_start..=x_end {
		for y in y_start..=y_end {
			if image.get_pixel_checked(x, y).unwrap_or(&Luma([0])).0[0] > 127 {
				return u8::MAX;
			}
		}
	}
	0
}
