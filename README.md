# tauri (v2) + wgpu + nokhwa demo

I needed an alternate way to efficiently render image buffers to my UI for [Helmer](https://www.helmer.app). Since Tauri's IPC and Websockets would be too slow, I decided to try something else.

One approach is to create WebGPU textures from the buffers and draw them directly to a native window. Luckily Tauri exposes that to us! This repo is a quick attempt to try it out, using the webcam as a source for the image buffers. It works quite well on Windows and Mac.

Interestingly, Tauri can also do multiple "surfaces" in the same window (as of v2). These could be Webviews or just regular things you'd chuck into a native window, allowing us a lot of room to play. Here's a Webview UI being rendered over a WebGPU texture in the same window.

<img width="1920" alt="Screenshot 2024-07-13 at 10 01 57 AM" src="https://github.com/user-attachments/assets/1c94221b-6c13-4a5b-9f4a-b0fe8a7dd912">

And Tauri can comfortably orchestrate interop between the two via events — nice!

## Development Guide

1. [Tauri prerequisites](https://beta.tauri.app/start/prerequisites/)
2. Run `pnpm install` to get deps
3. Run `pnpm run tauri dev` to build in dev mode

## Notes

1. On macOS, the formats reported by nokhwa (camera crate) don't match what's reported
2. Decoding of frames is done on cpu and is quite slow.
3. Demo app can be architected a lot better.

All easy fixes, but out of scope for my purposes. Feel free to PR them and I'll be happy to merge + credit you.

## References

1. [Introduction to WebGPU](https://www.youtube.com/watch?v=oIur9NATg-I)
2. [wgpu: Rust library for WebGPU](https://wgpu.rs)
3. [Learn wgpu](https://sotrh.github.io/learn-wgpu/)
4. [Learn wgpu: video playlist by chris biscardi](https://www.youtube.com/playlist?list=PLWtPciJ1UMuBs_3G-jFrMJnM5ZMKgl37H)
5. [Tauri and wgpu demo by FabianLars](https://github.com/FabianLars/tauri-v2-wgpu)
