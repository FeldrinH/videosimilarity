use std::env::args;

use anyhow::anyhow;
use ffmpeg_sidecar::{command::FfmpegCommand, event::{FfmpegEvent, LogLevel, OutputVideoFrame}};
use image::{save_buffer, ExtendedColorType};


fn save_frame(path: &str, frame: &OutputVideoFrame) -> Result<(), image::ImageError> {
    save_buffer(path, &frame.data, frame.width, frame.height, ExtendedColorType::Rgb8)
}

fn main() -> anyhow::Result<()> {
    let args = args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        return Err(anyhow!("expected 1 argument, got {}", args.len()));
    }

    let path = &args[0];

    ffmpeg_sidecar::download::auto_download()?;
    let ffmpeg_version = ffmpeg_sidecar::version::ffmpeg_version()?;
    println!("Using FFmpeg {}", ffmpeg_version);

    let iter = FfmpegCommand::new()
      .input(path)
      .rawvideo()
      .spawn()?
      .iter()?;

    for event in iter {
        match event {
            FfmpegEvent::Error(err) | FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, err) => {
                return Err(anyhow!(err).context("Failed to decode video with FFmpeg"));
            },
            FfmpegEvent::OutputFrame(frame) => {
                println!("frame {}: {}x{}", frame.frame_num, frame.width, frame.height);
                save_frame(&format!("frame_{}.png", frame.frame_num), &frame)?;
            }
            _ => {},
        }
    }

    Ok(())
}