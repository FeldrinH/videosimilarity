use std::{env::args, time::Instant};

use anyhow::{anyhow, bail};
use fast_image_resize::{images::{Image, ImageRef}, FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use ffmpeg_sidecar::{command::FfmpegCommand, event::{FfmpegEvent, LogLevel, OutputVideoFrame}};
use image::{save_buffer, ExtendedColorType, GrayImage, ImageError};
use image_hasher::{HashAlg, HasherConfig};

fn save_frame(path: &str, frame: &OutputVideoFrame) -> Result<(), ImageError> {
    save_buffer(path, &frame.data, frame.width, frame.height, ExtendedColorType::L8)
}

fn resize_frame(frame: &OutputVideoFrame, width: u32, height: u32) -> Image {
    let src_image = ImageRef::new(frame.width, frame.height, &frame.data, PixelType::U8).unwrap();
    let mut dst_image = Image::new(width, height, src_image.pixel_type());
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear));
    Resizer::new().resize(&src_image, &mut dst_image, &options).unwrap();
    dst_image
}

fn hash_frame(frame: &OutputVideoFrame) -> u64 {
    let image = resize_frame(frame, 9, 8);
    let image = GrayImage::from_raw(image.width(), image.height(), image.into_vec()).unwrap();
    let hasher = HasherConfig::with_bytes_type::<[u8; 8]>().hash_alg(HashAlg::Gradient).to_hasher();
    let hash = hasher.hash_image(&image);
    u64::from_le_bytes(hash.into_inner())
}

/// dHash based on https://www.hackerfactor.com/blog/index.php?/archives/529-Kind-of-Like-That.html
/// Should match HashAlg::Gradient from image_hasher
fn hash_frame_dhash(frame: &OutputVideoFrame) -> u64 {
    let image = resize_frame(frame, 9, 8);
    let pixels = image.buffer();
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            let bit = pixels[9 * y + x] < pixels[9 * y + x + 1];
            hash |= (bit as u64) << (8 * y + x)
        }
    }
    hash
}

fn main() -> anyhow::Result<()> {
    let args = args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        bail!("expected 1 argument, got {}", args.len());
    }

    let path = &args[0];

    ffmpeg_sidecar::download::auto_download()?;
    let ffmpeg_version = ffmpeg_sidecar::version::ffmpeg_version()?;
    println!("Using FFmpeg {}", ffmpeg_version);
    println!("Resizing CPU extensions: {:?}", Resizer::new().cpu_extensions());

    let start = Instant::now();

    let iter = FfmpegCommand::new()
      .input(path)
      //.rawvideo()
      .args(["-f", "rawvideo", "-pix_fmt", "gray", "-"])
      .spawn()?
      .iter()?;

    let mut frames = 0;
    let mut hashed = Vec::new();

    for event in iter {
        match event {
            FfmpegEvent::Error(err) | FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, err) => {
                return Err(anyhow!(err).context("Failed to decode video with FFmpeg"));
            },
            FfmpegEvent::OutputFrame(frame) => {
                frames += 1;
                let hash = hash_frame_dhash(&frame);
                if Some(&hash) != hashed.last() {
                    hashed.push(hash);
                }

                if frame.frame_num % 100 == 0 {
                    // save_frame(&format!("frame_{}.png", frame.frame_num), &frame)?;
                    println!("frame {}: {}x{} {:x}", frame.frame_num, frame.width, frame.height, hash);
                }
            }
            _ => {},
        }
    }

    let duration = start.elapsed();

    println!("{}/{}", hashed.len(), frames);
    println!("Time taken: {:?}", duration);

    Ok(())
}