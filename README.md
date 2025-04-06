[![Codacy Badge](https://app.codacy.com/project/badge/Grade/337033d4547044cf96a1584bf82b1ce8)](https://app.codacy.com/gh/Kek5chen/syrillian/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)

# Syrillian Engine

Syrillian Engine is a Rust-based, real-time 3D game engine built on top of [wgpu](https://github.com/gfx-rs/wgpu), focusing on flexibility, modularity, and a straightforward, entity-component-driven workflow. Designed to be easily extensible, Syrillian Engine aims to provide a robust foundation for building modern 3D applications, rendering pipelines, and post-processing effects.

## Features

- **Object Oriented Components**: Syrillian Engine provides a flexible OOP structure, allowing you to attach components (such as transforms, camera controllers, and custom logic) to game objects with minimal boilerplate.
- **Renderer Powered by wgpu**: Leverages the cross-platform [wgpu](https://github.com/gfx-rs/wgpu) API for graphics, giving you Vulkan/DX/Metal-level performance and portability.
- **Different Code Design Approach**: While other game engines focus on a very rust-based approach to their design, Syrillian focuses on ease of use and code simplicity.

## Getting Started

### Prerequisites

- **Rust & Cargo**: Ensure you have the latest stable Rust toolchain installed. A nightly compiler toolchain might be necessary for some builds.
  [Install Rust](https://www.rust-lang.org/tools/install)
- **wgpu-compatible GPU**: Syrillian Engine uses wgpu, which requires a modern graphics API (Vulkan, Metal, DX12, or WebGPU).

### Building & Running

1. Clone the repository:
   ```bash
   git clone https://github.com/Kek5chen/syrillian.git
   cd syrillian
   ```

2. Build the engine:
   ```bash
   cargo build
   ```

3. Run a demo or test application included in the repository:
   ```bash
   cargo run --example my-main
   ```

**NixOS** *Development Flakes are provided with the project.*

If successful, a window should appear displaying a rendered scene.

### Project Structure

- **`src/`**: The core engine code.
- **`shaders/`**: WGSL shader files for main 3D rendering and post-processing passes.
- **`examples/`**: Example applications or scenes demonstrating usage of the engine.

### Customizing the Engine

- **Add New Shaders**: Place new WGSL shaders in the `shaders/` directory and register them in the `ShaderManager`.
- **Add Components & Systems**: Extend `components/` with new component types, and integrate custom logic in ECS update steps.
- **Add Post-Processing Effects**: Create off-screen passes and write new fragment shaders for effects. Then hook them into the `Renderer`’s second pass.

If you are planning the use the engine as a library and not to extend the engine itself, consider adopting a similar structure in your project.

### Roadmap

- **Additional Render Features**: Shadows, PBR materials, Skyboxes, HDR and bloom are planned enhancements.
- **Editor Tools**: Add an editor UI for placing objects, tweaking materials, and viewing engine stats.

### Contributing

Contributions are welcome! If you find a bug or have a feature request:

1. Open an issue describing the problem or feature.
2. Discuss solutions or improvements.
3. Submit a pull request with your changes.

Ensure your code follows Rust’s formatting and clippy checks:
```bash
cargo fmt
cargo clippy
```

### License

Syrillian Engine is distributed under the MIT License. See [LICENSE](LICENSE) for details.

---

**Syrillian Engine**: Building the backbone of your next great 3D experience.
