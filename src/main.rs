use std::{env::args, time::Instant};

use anyhow::{anyhow, bail};
use fast_image_resize::{images::{Image, ImageRef}, FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use ffmpeg_sidecar::{command::FfmpegCommand, event::{FfmpegEvent, LogLevel, OutputVideoFrame}};
use image::{save_buffer, ExtendedColorType, GrayImage, ImageError};
use image_hasher::{HashAlg, HasherConfig};

fn save_frame(path: &str, frame: &OutputVideoFrame) -> Result<(), ImageError> {
    save_buffer(path, &frame.data, frame.width, frame.height, ExtendedColorType::L8)
}

fn hash_frame(frame: &OutputVideoFrame) -> u64 {
    let buffer = ImageRef::new(frame.width, frame.height, &frame.data, PixelType::U8).unwrap();
    let mut resized_buffer = Image::new(9, 8, buffer.pixel_type());
    Resizer::new().resize(&buffer, &mut resized_buffer, &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))).unwrap();

    // TODO: Implement this code without image_hasher library
    let resized_buffer = GrayImage::from_raw(resized_buffer.width(), resized_buffer.height(), resized_buffer.into_vec()).unwrap();
    let hasher = HasherConfig::with_bytes_type::<[u8; 8]>().hash_alg(HashAlg::Gradient).to_hasher();
    let hash = hasher.hash_image(&resized_buffer);
    u64::from_le_bytes(hash.into_inner())
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
                let hash = hash_frame(&frame);
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