mod template;
mod video;
use clap::{arg, Parser};

#[derive(Parser, Debug)]
struct Args {
	#[arg(short, long, default_value = "1")]
	window: u32,
	#[arg(short, required = true)]
	input: String,
	#[arg(long)]
	noloop: bool,
	#[arg(long)]
	invert: bool,
	#[arg(long, default_value = "1")]
	upscale: u8,

	#[arg(long, default_value = "24")]
	fps: u32,

	#[arg(long)]
	cutoff: Option<u8>,
	#[arg(short, required = true)]
	output: String,
}

fn main() {
	let args = Args::parse();
	let template = template::Template::new(&args).unwrap();
	if args.window > 1 {
		template.expand_black_by(args.window);
	}
	let video = video::Video::new(&args, &template).unwrap();
	video.render();
	video.compile();
}
