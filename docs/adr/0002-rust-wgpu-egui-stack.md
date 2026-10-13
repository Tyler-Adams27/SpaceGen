# Rust with wgpu and egui

SpaceGen is a Rust desktop application using `wgpu` for GPU rendering (shaders in WGSL) and `egui` for the GUI. This gives a single cross-platform shader language and a single-binary Windows build. Windows is the primary target; Mac and Linux are out of scope for version 1 but not blocked by the stack.

## Considered Options

- **C++ with Qt or Dear ImGui plus OpenGL/Vulkan**: mature, but heavier build and tooling.
- **C# with Avalonia or WPF**: productive GUI, weaker fit for a portable GPU shader pipeline.
- **TypeScript with Electron or Tauri and WebGL/WebGPU**: easy distribution, but large runtime and constrained GPU control for 8K tiled export.
- **Python with PySide and moderngl**: fastest to write, hardest to ship.
