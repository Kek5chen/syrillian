[![Codacy Badge](https://app.codacy.com/project/badge/Grade/337033d4547044cf96a1584bf82b1ce8)](https://app.codacy.com/gh/Kek5chen/syrillian/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![codecov](https://codecov.io/github/kek5chen/syrillian/graph/badge.svg?token=QORLO7MO2I)](https://codecov.io/github/kek5chen/syrillian)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/Kek5chen/syrillian)
[![Discord](https://img.shields.io/discord/1401869988796698696?style=flat&label=Discord)](https://discord.gg/hZWycSwSm4)

# Syrillian Engine

Syrillian Engine is a Rust-based, real-time 3D game engine, focusing on flexibility, modularity, and a straightforward,
entity-component-driven workflow.

It's designed to be easily simple and extensible. We aim to provide a robust foundation for building modern 3D
applications, rendering pipelines, and visually pleasing gameplay.

## We're doing it different!

**We're unique**, in the sense that we aim to show how *flexible* Rust can be as a language. With the milestone to
provide
a **simple, iteration-strong game engine**, which people **have fun** making games with. The goal is to look beyond the
boundaries so that users can simply focus on iteration speed. **Not fighting a language.**

The goal is that even new developers, and people familiar with other languages, have a comfortable dip into the Rust
game-dev atmosphere!

### Showroom

**Feel free to add your own expositions here :)**

![](https://i.ibb.co/F9gywNk/Screenshot-2025-08-04-at-12-37-22.png)

*Me picking up a shader-animated, physics-enabled cube which is emitting a
lightsource* [From this Example](./examples/my-main.rs)

## Features

- Simple "Just get it started" approach. High focus on user-side simplicity, and even fresh rust users should feel
  welcome.
- Lots of preset (components - prefabs - compatibility)! All boxed up, ready to be dropped in.
- Mesh and Physics, Visual debugging features.
- Game Objects that are *builder extensible*. Providing a fluid object creation and behavior specification workflow.
- Built in giants:
    - Strong battle-tested Physics Integration provided by [rapier](https://github.com/dimforge/rapier)
    - Ultra dynamic Graphics Abstraction provided by [wgpu](https://github.com/gfx-rs/wgpu) (DirectX, Vulkan, Metal,
      WebGL)

## Getting Started

### Prerequisites

- Have a Rust Toolchain Installed.
- Have a GPU

### Building & Running

1. Clone the repository:
   ```bash
   git clone https://github.com/Kek5chen/syrillian.git
   cd syrillian
   ```

2. Build the engine library:
   ```bash
   cargo build
   ```

3. Try out a demo example, included in the repository:
   ```bash
   cargo run --example my-main
   ```

4. Dance in circles (optional)

**NixOS** *Development Flakes are provided with the project.*

If successful, a window should appear displaying a rendered scene.

### Minimal Setup

We, optionally, provide
the [SyrillianApp Proc Macro](https://docs.rs/syrillian_macros/latest/syrillian_macros/derive.SyrillianApp.html).
Usage example:

```rust
// The macro will provide you with a simple main runtime and (optional) logging
#[derive(Debug, Default, SyrillianApp)]
struct YourGame;

impl AppState for YourGame {
    // will be called once
    fn init(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> {
        window.set_title("Example App");

        world.new_camera();
        world.spawn(&CubePrefab).at(0, 0, -10); // Spawn Cube at (0, 0, -10).
        world.print_objects(); // Print Scene Hierarchy to Console

        Ok(())
    }

    // will be called once per frame
    fn update(&mut self, world: &mut World, window: &Window) -> Result<(), Box<dyn Error>> { ... }

    // there's also "late_update", "draw", and more...
}
```

It's simple... Really!

### Roadmap & Contributing

The whole feature map has moved into
the [Issues](https://github.com/Kek5chen/syrillian/issues?q=state%3Aopen%20label%3Aepic)

Contributions are welcome! If you find a bug or have a feature request:

1. Open an issue describing the problem or feature.
2. Discuss solutions or improvements.
3. Optionally, submit a pull request with your changes. Very welcome!

Ensure your code follows Rust’s formatting and clippy checks:

```bash
cargo fmt
cargo clippy
```

### History

This project has been a big personal gem of mine, and I've poured probably a good 100+ hours of my freetime into it.
I am not monetized to work on this, and I do it out of pure personal interest.

**Any help**, getting this project [better, stable, improved, ..] is very welcome, and I'll make sure to show or explain
anything that's not clear. Even feedback or rants on the user-facing API are more than welcome, as I wish to provide
patterns that make is as simple as possible to use.

If you need *anything*: Don't hesitate to join the [discord](https://discord.gg/hZWycSwSm4) to discuss :)

### License

Syrillian Engine is distributed under the MIT License. See [LICENSE](LICENSE) for details.

---

Syrillian Engine ❤️ Building the backbone of your next great 3D experience.
