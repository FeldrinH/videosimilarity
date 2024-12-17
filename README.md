FFmpeg decoding test using Rust.

### Running

Install Rust (https://rust-lang.org/), then run the following:

```bash
$ cargo run -- myvideo.mp4
```

Warning: This will save the frames of `myvideo.mp4` as PNG images, which may use a lot of disk space if the video is large. 