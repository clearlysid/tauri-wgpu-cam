# tauri (v2) + wgpu + nokhwa demo

I needed an alternate way to render image/video buffers to my UI efficiently for [Helmer](https://www.helmer.app). Since Tauri's IPC would be too slow and Websockets would tend to choke with the amount of data we'd have to pipe, I decided to try something else.

One approach was to create WebGPU textures from the buffers and draw them directly to a native window. Luckily Tauri exposes that to us! This repo is my quick attempt to try it, using the webcam as a source for the image buffers. It works well on macOS and Windows.

Interestingly, Tauri can also now do multiple "surfaces" in the same window since v2. These surfaces could be Webviews or just regular things you would chuck into a native window, allowing us quite a lot of room for experimentation. Here's a sample of a UI running in Webview rendered over a WebGPU texture in the same window.

<img width="1920" alt="Screenshot 2024-07-13 at 10 01 57 AM" src="https://github.com/user-attachments/assets/1c94221b-6c13-4a5b-9f4a-b0fe8a7dd912">

And Tauri can comfortably orchestrate interop between the two via events — nice!

## Notes

1. On macOS, the formats reported by the camera capture crate don't match what's reported and the decoding of frames is quite slow.
2. This demo can be architected a lot better.

Both are fairly straightforward fixes, but out of scope for my work on this repo. Feel free to PR them if you like and I'll be happy to merge it and credit you.

### References

1. [Introduction to WebGPU](https://www.youtube.com/watch?v=oIur9NATg-I)
2. [wgpu: Rust library for WebGPU](https://wgpu.rs)
3. [Learn wgpu](https://sotrh.github.io/learn-wgpu/)
4. [Learn wgpu: video playlist by chris biscardi](https://www.youtube.com/playlist?list=PLWtPciJ1UMuBs_3G-jFrMJnM5ZMKgl37H)
5. [Tauri and wgpu demo by FabianLars](https://github.com/FabianLars/tauri-v2-wgpu)
