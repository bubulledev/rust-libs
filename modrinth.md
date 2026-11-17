# Modrinth listing — RustLib

Everything below is ready to paste into Modrinth's project creation flow.
The French draft is kept in `modrinth.fr.md`.

## Metadata to fill in

| Field | Value |
| --- | --- |
| Slug | `rustlib` |
| Project name | RustLib |
| Project type | Mod |
| Summary | see below (238 characters) |
| Loaders | Fabric |
| Game versions | 1.21.x (the bridge references no Minecraft class: compatibility comes from your mod) |
| Categories | Library, Utility |
| License | `MIT OR Apache-2.0` → pick "Custom license" with that text, or "MIT" if one is enough |
| Links | Wiki: your wiki; Source: your Git repository |
| Version file | `dist/rustlib-0.1.0.jar` (one jar per platform: Windows, Linux, macOS) |

## Summary field

> Library for writing the logic of your Fabric mods in Rust: registries,
> prioritised events, tick work bounded by a time budget, chunk access. Shipped
> as a jar with its native library embedded.

## Description (page body)

# RustLib — your Fabric mod's logic, in Rust

**RustLib is not a gameplay mod: it is a library for mod developers.** It
provides the Rust half of a Fabric mod: block and item registries, an event bus,
a tick scheduler bounded by a time budget, and a chunk-section cache. Rendering,
networking and vanilla logic stay in Java.

If you are a player looking for a mod to install: **this is not it.** RustLib
does not run on its own and adds no content.

## The problem

A Minecraft mod runs on the server thread, where 50 ms per tick must cover
everything. As soon as a mod does heavy computation — machine simulation,
pathfinding, whole-chunk processing — it eats that budget and the server starts
to lag. Such code also tends to reinvent its own scheduler, block cache and
priority handling, allocating all over the tick.

## What RustLib gives you

| Piece | What it solves |
| --- | --- |
| **Registries** | An identifier cannot be registered twice, and registries freeze once loading ends, so you never ship a world the game cannot read back. |
| **Events** | Five priority levels, deterministic execution order, cancellation (breaking a block, taking damage). Standard events included: `BlockBreak`, `BlockPlace`, `PlayerDamage`, `ServerTick`, `PlayerJoin`, `PlayerQuit`. |
| **Bounded ticks** | A time budget per tick: leftover work is **deferred** to the next tick instead of blowing the tick up. Missed deadlines are never replayed in a burst. |
| **World data** | Dense chunk sections (4096 states as `u16`, 8 KiB), non-air counter in O(1), cache filled once from the JVM. |
| **Declarative content** | Blocks and items described in Rust (material, hardness, resistance, light, drops, tools, durability, food), then exported as JSON into the Java registry at load time. |

The core (`rustlib-core`) has **no external dependency**: no `serde`, no async
runtime, no math crate. You keep full control over what gets compiled and what
runs inside the tick.

## How it works

```text
rustlib.jar (JVM) ── one JNI call per tick ──▶ Rust runtime
   RustLib.class                               events
   embedded native library                     spatial queries
   fabric.mod.json                             bounded simulations
```

The JNI boundary is expensive: one round trip per block access would make your
mod slower than its Java version. RustLib is therefore built around a single
rule: **one call per tick**, with data copied once (registries as JSON at load
time, chunk sections when they load).

## Installation (developers)

1. Add `rustlib-core` and `rustlib-jni` to your mod crate.
2. Export your mod to Java with a single line:

   ```rust
   rustlib_jni::export_mod!(|| Box::new(MyMod::new()));
   ```

3. Build the jar (the script runs Rust, javac, assembly and verification):

   ```bash
   ./build-jar.sh     # → dist/rustlib-0.1.0.jar
   ```

4. Drive it from your Fabric entrypoint:

   ```java
   RustLib lib = RustLib.start(2_000_000L);   // 2 ms budget per tick
   String json = lib.registriesJson();        // blocks and items to create
   lib.tick();                                // once per tick
   lib.blockBreak(x, y, z, "copper:copper_ore", "Alex");
   lib.close();
   ```

The jar contains `dev/rustlib/RustLib.class`, a `fabric.mod.json` (a library
with no entrypoint) and your mod's native library. Put it in `mods/`, declare it
as a nested jar inside a Fabric mod, or use it outside Minecraft to test the
logic.

## What RustLib does not do

* **No rendering, no client side.** No access to shaders, models, particles or
  input: the target is server logic.
* **No mixins from Rust.** Rust cannot rewrite Java bytecode: your hooks go
  through the Fabric APIs (or through a Java mixin that calls the bridge).
* **No magic speedup.** Moving code to Rust only speeds up what was genuinely
  costly and badly optimised. Heavy computation on data already copied wins;
  one-off calls into the Minecraft API lose.
* **No persistence.** Rust state does not survive a restart; use Minecraft's own
  save structures on the Java side.
* **One jar per platform.** A Windows `.dll` will not load on Linux.

## An example is included

The repository ships **RustLib Mods**, a small demo mod: two blocks, two
items, one tool, and a quota of 64 ore breaks per 100-tick window that
demonstrates a Rust handler cancelling an event. All of its logic is tested
without a Minecraft server.

## Quality

* 58 automated tests (registries, priorities, cancellation, tick budget with
  deferral, chunk sections, registry JSON).
* A smoke test that loads the jar, extracts its native library, starts the mod,
  sends ticks and checks a cancelled block break — without Minecraft.
* Explicit errors: invalid identifier, duplicate, frozen registry, reserved
  namespace.

## Status

**Alpha 0.1.0.** The API may change. The Java side of the Fabric integration is
documented and tested outside the game (JNI bridge included); wiring the Fabric
events is up to your mod. Shipped for Windows, Linux and macOS depending on the
build platform.

## Links

* Wiki: 10 pages, from "first mod" to the JNI/Fabric bridge and the limits.
* Source: your Git repository.

## Changelog 0.1.0

First public release.

* Dependency-free Rust core: validated identifiers, typed registries with
  freezing, event bus with priorities and cancellation, tick scheduler with a
  time budget, dense chunk sections, registry JSON export.
* `rustlib.jar`: `dev.rustlib.RustLib` class, `fabric.mod.json`, native library
  embedded and loaded straight from the jar.
* `export_mod!` macro: five JNI symbols with fixed names, one jar for every mod.
* RustLib Mods example mod and 58 automated tests.

## Publishing notes

* Icon: 512 × 512 px PNG (an iron-coloured gear, for example).
* Gallery: a screenshot of the registry JSON, a wiki excerpt (events page), and
  the "one JNI call per tick" diagram.
* Honest reminder: the listing states plainly that this is a developer library
  and that wiring it into Fabric is the mod author's job. A library that
  under-promises ages far better than one that promises a working mod.
