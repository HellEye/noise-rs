use std::{path::Path, process::Command};

use image::{GrayImage, ImageBuffer, Luma, Pixel};
use rand::Rng;

use crate::{
	template::{Template, TEMPLATE_FILE_EXTENSION},
	Args,
};

pub struct Video<'a> {
	tmp_path: String,
	path: String,
	template: &'a Template,
	args: &'a Args,
}
#[derive(Debug)]
pub enum VideoCreateError {
	InputError(String),
	UnknownError(String),
}

impl From<std::io::Error> for VideoCreateError {
	fn from(e: std::io::Error) -> Self {
		VideoCreateError::UnknownError(e.to_string())
	}
}
pub const FRAME_DIR_NAME: &str = "frames";
pub const FRAME_FILE_NAME: &str = "frame";
fn create_noisy(width: u32, height: u32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
	let mut image: ImageBuffer<Luma<u8>, Vec<u8>> = GrayImage::new(width, height);
	let mut rng = rand::thread_rng();
	for px in image.pixels_mut() {
		if rng.gen_bool(0.5) {
			px.0[0] = u8::MAX;
		} else {
			px.0[0] = 0;
		}
	}

	image
}
type FrameBuffer = ImageBuffer<Luma<u8>, Vec<u8>>;

impl<'a> Video<'a> {
	pub fn get_ffmpeg_name() -> String {
		let tmp_dir = Template::get_tmp_dir();
		tmp_dir
			.join(FRAME_DIR_NAME)
			.join(format!(
				"{}%04d.{}",
				FRAME_FILE_NAME, TEMPLATE_FILE_EXTENSION
			))
			.to_str()
			.unwrap()
			.to_string()
	}
	pub fn get_file_name(frame: usize) -> String {
		format!(
			"{}{:04}.{}",
			FRAME_FILE_NAME, frame, TEMPLATE_FILE_EXTENSION
		)
	}
	pub fn new(args: &'a Args, template: &'a Template) -> Result<Self, VideoCreateError> {
		let current_dir = std::env::current_dir()?;
		let input_file_path = match current_dir.join(args.input.as_str()).to_str() {
			Some(input) => input.to_string(),
			None => {
				return Err(VideoCreateError::InputError(
					"Could not convert path to string".into(),
				))
			}
		};
		let file = Path::new(input_file_path.as_str());
		if !file.exists() {
			return Err(VideoCreateError::InputError(format!(
				"File {} does not exist",
				input_file_path
			)));
		}

		let out_path = current_dir
			.join(args.output.as_str())
			.to_str()
			.unwrap()
			.to_string();
		let global_tmp_dir = std::env::temp_dir();
		let tmp_dir = global_tmp_dir.join("noise").join(FRAME_DIR_NAME);
		if !tmp_dir.exists() {
			std::fs::create_dir_all(&tmp_dir)?;
		}
		Ok(Video {
			tmp_path: tmp_dir.to_str().unwrap().to_string(),
			path: out_path,
			template,
			args,
		})
	}
	pub fn render(&self) {
		println!("Creating noise image with size: {:?}", self.template.size);
		let mut image = create_noisy(self.template.size.0, self.template.size.1);
		for i in 0..(if self.args.noloop { 1 } else { 2 }) {
			for (frame, template) in self.template.image_paths.iter().enumerate() {
				let frame = frame + (i * self.template.image_paths.len());
				self.render_single(&mut image, frame, template.clone());
			}
		}
	}
	fn should_flip(&self, px_value: u8) -> bool {
		let mut rng = rand::thread_rng();

		match self.args.cutoff {
			Some(cutoff) => {
				if (!self.args.invert && px_value < cutoff)
					|| (self.args.invert && px_value > cutoff)
				{
					return true;
				}
			}
			None => {
				let flip_chance = ((if self.args.invert {
					px_value as f32
				} else {
					255.0 - px_value as f32
				}) / 255.0)
					.powi(3);
				if rng.gen_bool(flip_chance as f64) {
					return true;
				}
			}
		}
		false
	}
	fn render_single(&self, image: &mut FrameBuffer, frame: usize, template: String) {
		let template = image::open(template)
			.unwrap_or_else(|_| panic!("Could not open template image for frame {}", frame))
			.to_luma8();
		for (x, y, px) in image.enumerate_pixels_mut() {
			let template_px = template.get_pixel(
				(x as f32 / self.args.upscale as f32).floor() as u32,
				(y as f32 / self.args.upscale as f32).floor() as u32,
			);
			if self.should_flip(template_px.0[0]) {
				px.invert()
			}
		}
		let tmp_path = Path::new(&self.tmp_path);
		let file_name = Video::get_file_name(frame);
		let binding = tmp_path.join(file_name.as_str());
		let save_path = binding.to_str().unwrap();
		image
			.save(save_path)
			.unwrap_or_else(|_| panic!("Could not save frame {}", frame));
	}

	pub fn compile(&self) {
		println!("Compiling with ffmpeg");
		println!("size: {:?}", self.template.size);
		let out_res = Command::new("ffmpeg")
			.args([
				"-y",
				"-framerate",
				self.args.fps.to_string().as_str(),
				"-i",
				Video::get_ffmpeg_name().as_str(),
				"-c:v",
				"libx264",
				"-pix_fmt",
				"yuv420p",
				self.path.as_str(),
			])
			.output();
		match out_res {
			Ok(out_res) => {
				if !out_res.status.success() {
					eprintln!(
						"ffmpeg build video output: \n{}\n stderr: {}",
						String::from_utf8_lossy(&out_res.stdout),
						String::from_utf8_lossy(&out_res.stderr)
					);
				}
			}
			Err(e) => {
				eprintln!("ffmpeg build video failed: \n{}", e);
			}
		}
	}
}

impl Drop for Video<'_> {
	fn drop(&mut self) {
		std::fs::remove_dir_all(&self.tmp_path).unwrap();
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
