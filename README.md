# Noise gif generator

Converts black and white gifs into noise-based persistence of vision illusions.

Inspired by [Weird POV effect](https://www.youtube.com/watch?v=TdTMeNXCnTs) on youtube

## Usage

- `cargo build --release` to build the binary (release flag recommended for much better performance)
- `cargo run --release -- -i input.gif -o output.gif` to convert the gif

Requires ffmpeg to be installed

## Parameters

Required:

- `-i` input file
- `-o` output file - uses ffmpeg under the hood, so most video extensions should work, recommended .mp4

Optional:

- `-w <number>`, `--window <number>` window size - increases the size of pixel detection. Useful for gifs with thin lines, but has high performance impact (default 1)
- `--noloop` no loop - do not loop the gif a second time.
  - due to how the program works, without looping there will be a noticeable jump when the gif is played and ends. Because of that, the generated output by default is two loops to cancel out the jump. This flag skips the second loop
- `--invert` invert - invert the image (output will show white parts)
- `--upscale <number>` upscale - upscaling factor (default 1)
  - this will generate the output with higher resolution than the input
  - Has high performance impact, and very high file size impact
- `--fps <number>` - output file frames per second (default 24)
- `--cutoff <number>` - boundry between black/white.
  - Without specifying a cutoff value, gray pixels will be switched randomly based on their value. This could lead to clearer images, but also to a noticeable jump when the gif ends and starts again.
  - If you specify a value, only pixels with values in the range from 0 (or 255 with invert) to the cutoff value will be switched. This will result in a more uniform image, but will lead to loss of grayscale information.
