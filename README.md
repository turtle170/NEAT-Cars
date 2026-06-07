# PPO Cars — Rust Edition 🦀

A rewrite of PPO Cars using **Bevy 0.18**, **bevy_rapier3d 0.34**, and **burn 0.21** (NdArray backend).

## Stack

| | Library | Purpose |
|---|---|---|
| 🎮 | `bevy 0.18` | ECS game engine |
| ⚙️ | `bevy_rapier3d 0.34` | 3D rigid body physics, wheel joints |
| 🧠 | `burn 0.21` (NdArray) | PPO Actor-Critic training (pure Rust, no C++) |

## Features

- **10×10×10 voxel cars** with 22 block types and special abilities
- **16 weapons**: RPG, Minigun, Self-Aiming Turret, Flamethrower, EMP, Laser, Grenade, Shotgun, Railgun, Homing Missile, Mine Layer, Tesla Coil, Plasma Cannon, Mortar, Sticky Bomb, Drill
- **Per-agent PPO policy** — each of the 10-15 AIs trains its own independent network
- **Mesh vertex deformation** — blocks crumple and bend on damage before breaking
- **Rocket League–style arena** — curved walls, ramps, downforce physics
- **Spectator freecam** — orbit, zoom, WASD pan, Tab to cycle agents, F to release

## Building & Running

```powershell
# Debug (fast compile, slower runtime)
cargo run

# Release (slower compile, fast runtime — use for long training)
cargo run --release

# Enable GPU training (edit Cargo.toml: uncomment burn-wgpu, add feature flag)
cargo run --release --features gpu
```

## Controls

| Input | Action |
|---|---|
| Right-click + drag | Orbit camera |
| Scroll wheel | Zoom |
| WASD | Pan (free mode) |
| Tab | Cycle to next car |
| F | Release follow target |
| F1 | Toggle Rapier debug rendering |

## Architecture

```
src/
├── main.rs              — App setup + arena construction
├── voxel/
│   ├── block.rs         — 22 block types (static table, zero-cost)
│   ├── grid.rs          — VoxelGrid ECS component (10×10×10)
│   ├── builder.rs       — genome → car + BFS connectivity
│   └── deformer.rs      — Perlin-noise vertex deformation
├── physics/
│   └── car_controller.rs — Rapier MultibodyJoint wheel drive
├── weapons/
│   ├── mod.rs            — WeaponType, WeaponState, MountedWeapons
│   ├── systems.rs        — All 16 weapon fire implementations
│   ├── projectile.rs     — Physics projectile lifecycle
│   ├── homing.rs         — Missile guidance system
│   └── mine.rs           — Proximity mine arm/trigger
├── ai/
│   ├── agent.rs          — CarAgent + 62-obs builder + action applier
│   ├── network.rs        — burn Actor-Critic MLP
│   ├── ppo.rs            — PPO update (clip + entropy + GAE)
│   ├── replay_buffer.rs  — Rolling trajectory buffer + GAE computation
│   └── mod.rs            — AiPlugin + per-agent trainer pool
├── battle/
│   ├── manager.rs        — Build→Battle→Reset state machine
│   └── damage.rs         — Splash damage events + fragment spawning
├── camera/freecam.rs     — Spectator orbit camera
└── ui/hud.rs             — Bevy UI health bars + episode info
```

## Training Notes

Training happens **in-process** inside Bevy — no external Python needed!

Each agent has its own `PpoTrainer<Autodiff<NdArray>>`. When an agent's rolling buffer
fills (2048 steps), a PPO update runs synchronously on the Bevy `Update` schedule.

For GPU training, switch to the `burn-wgpu` backend by:
1. Uncomment `burn-wgpu` in `Cargo.toml`
2. Change `NdArray` → `Wgpu` in `src/ai/mod.rs`

## License
Apache 2.0
