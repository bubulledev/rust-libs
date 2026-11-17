# 9. Référence API

Toutes les signatures publiques, par module. Les types du préambule sont
disponibles via `use rustlib::prelude::*;`.

## `rustlib::id`

```rust
pub const MINECRAFT_NAMESPACE: &str = "minecraft";
pub const RUSTLIB_NAMESPACE: &str = "rustlib";

pub enum IdError { Empty { part: &'static str },
                   InvalidCharacter { part: &'static str, ch: char },
                   TooManySeparators }

pub struct ResourceLocation { /* privé */ }

impl ResourceLocation {
    pub fn new(namespace: impl AsRef<str>, path: impl AsRef<str>) -> Result<Self, IdError>;
    pub fn mc(namespace: impl AsRef<str>, path: impl AsRef<str>) -> Self;
    pub fn parse(raw: &str) -> Result<Self, IdError>;
    pub fn namespace(&self) -> &str;
    pub fn path(&self) -> &str;
    pub fn is_minecraft(&self) -> bool;
    pub fn with_namespace(&self, namespace: impl AsRef<str>) -> Result<Self, IdError>;
    pub fn path_starts_with(&self, prefix: &str) -> bool;
}
// + Display, Debug, Clone, PartialEq, Eq, Ord, Hash, FromStr
```

## `rustlib::manifest`

```rust
pub struct Version;
impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self;
    pub fn parse(raw: &str) -> Result<Self, VersionError>;
    pub const fn major(self) -> u32;
    pub const fn minor(self) -> u32;
    pub const fn patch(self) -> u32;
}
// + Display, Ord, Hash, FromStr

pub enum VersionError { Malformed, NotANumber }
pub enum ManifestError { EmptyName, InvalidId(IdError), ReservedNamespace(ResourceLocation) }

pub struct ModMetadata;
impl ModMetadata {
    pub fn builder(id: ResourceLocation, name: impl Into<String>, version: Version)
        -> ModMetadataBuilder;
    pub fn id(&self) -> &ResourceLocation;
    pub fn namespace(&self) -> &str;
    pub fn name(&self) -> &str;
    pub fn version(&self) -> Version;
    pub fn authors(&self) -> &[String];
    pub fn description(&self) -> &str;
    pub fn license(&self) -> Option<&str>;
    pub fn minecraft_range(&self) -> Option<&str>;
}

pub struct ModMetadataBuilder;
impl ModMetadataBuilder {
    pub fn author(self, author: impl Into<String>) -> Self;
    pub fn authors(self, authors: impl IntoIterator<Item = impl Into<String>>) -> Self;
    pub fn description(self, description: impl Into<String>) -> Self;
    pub fn license(self, license: impl Into<String>) -> Self;
    pub fn minecraft_versions(self, range: impl Into<String>) -> Self;
    pub fn build(self) -> Result<ModMetadata, ManifestError>;
}
```

## `rustlib::registry`

```rust
pub trait RegistryEntry { fn id(&self) -> &ResourceLocation; }

pub enum RegistryError { Duplicate(ResourceLocation), Frozen }

pub struct Registry<T>;
impl<T> Registry<T> {
    pub fn new() -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn is_frozen(&self) -> bool;
    pub fn freeze(&mut self);
    pub fn iter(&self) -> impl Iterator<Item = &T>;
}
impl<T: RegistryEntry> Registry<T> {
    pub fn register(&mut self, entry: T) -> Result<(), RegistryError>;
    pub fn get(&self, id: &ResourceLocation) -> Option<&T>;
    pub fn contains(&self, id: &ResourceLocation) -> bool;
    pub fn ids(&self) -> impl Iterator<Item = &ResourceLocation>;
}
```

## `rustlib::event`

```rust
pub trait Event: 'static {
    fn is_cancelled(&self) -> bool { false }
    fn cancel(&mut self) {}
}

pub enum EventPriority { Lowest, Low, Normal, High, Highest }  // Default = Normal
pub struct HandlerId(u64);

pub struct EventBus;
impl EventBus {
    pub fn new() -> Self;
    pub fn subscribe<E: Event>(&mut self, priority: EventPriority,
        handler: impl FnMut(&mut E) + Send + 'static) -> HandlerId;
    pub fn subscribe_including_cancelled<E: Event>(&mut self, priority: EventPriority,
        handler: impl FnMut(&mut E) + Send + 'static) -> HandlerId;
    pub fn post<E: Event>(&mut self, event: &mut E) -> bool;
    pub fn unsubscribe<E: Event>(&mut self, id: HandlerId) -> bool;
    pub fn handler_count<E: Event>(&self) -> usize;
    pub fn total_handlers(&self) -> usize;
}
```

## `rustlib::events`

```rust
pub struct BlockBreak  { pub block: ResourceLocation, pub pos: BlockPos,
                         pub player: String, pub cancelled: bool }   // annulable
pub struct BlockPlace  { pub block: ResourceLocation, pub pos: BlockPos,
                         pub player: String, pub cancelled: bool }   // annulable
pub struct PlayerDamage{ pub player: String, pub amount: f32,
                         pub cancelled: bool }                       // annulable
pub struct ServerTick  { pub tick: u64 }
pub struct PlayerJoin  { pub player: String }
pub struct PlayerQuit  { pub player: String }

impl BlockBreak   { pub fn new(block: ResourceLocation, pos: BlockPos, player: impl Into<String>) -> Self; }
impl BlockPlace   { pub fn new(block: ResourceLocation, pos: BlockPos, player: impl Into<String>) -> Self; }
impl PlayerDamage { pub fn new(player: impl Into<String>, amount: f32) -> Self; }
```

## `rustlib::tick`

```rust
pub struct TaskId(u64);
pub struct TickContext<'a> { pub tick: u64, pub task: &'a str,
                             pub budget_remaining: Duration }
pub struct TickReport { pub tick: u64, pub executed: usize, pub deferred: usize,
                        pub overrun: bool, pub elapsed: Duration }
impl TickReport { pub fn is_healthy(&self) -> bool; }

pub struct TickScheduler;
impl TickScheduler {
    pub fn new(budget: Duration) -> Self;
    pub fn budget(&self) -> Duration;
    pub fn set_budget(&mut self, budget: Duration);
    pub fn current_tick(&self) -> u64;
    pub fn pending(&self) -> usize;
    pub fn schedule_after(&mut self, delay_ticks: u64, name: &'static str,
        run: impl FnMut(&mut TickContext<'_>) + Send + 'static) -> TaskId;
    pub fn schedule_repeating(&mut self, period_ticks: u64, name: &'static str,
        run: impl FnMut(&mut TickContext<'_>) + Send + 'static) -> TaskId;
    pub fn cancel(&mut self, id: TaskId) -> bool;
    pub fn run(&mut self, tick: u64) -> TickReport;
}
```

## `rustlib::world`

```rust
pub type BlockStateId = u16;
pub const AIR: BlockStateId = 0;
pub const SECTION_SIZE: usize = 16;
pub const SECTION_VOLUME: usize = 4096;
pub const fn section_index(x: usize, y: usize, z: usize) -> usize;

pub struct SectionPos { pub chunk_x: i32, pub section_y: i32, pub chunk_z: i32 }
impl SectionPos {
    pub const fn new(chunk_x: i32, section_y: i32, chunk_z: i32) -> Self;
    pub fn from_block(pos: BlockPos) -> Self;
}

pub struct ChunkSection;
impl ChunkSection {
    pub fn new() -> Self;
    pub fn from_states(states: &[BlockStateId]) -> Self;
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockStateId;
    pub fn set(&mut self, x: usize, y: usize, z: usize, state: BlockStateId);
    pub fn non_air_count(&self) -> u32;
    pub fn is_empty(&self) -> bool;
    pub fn fill(&mut self, state: BlockStateId);
    pub fn as_states(&self) -> &[BlockStateId; SECTION_VOLUME];
}

pub trait WorldView {
    fn block_state_at(&self, pos: BlockPos) -> Option<BlockStateId>;
    fn is_loaded(&self, pos: BlockPos) -> bool;   // défaut : block_state_at().is_some()
}

pub struct SectionCache;
impl SectionCache {
    pub fn new() -> Self;
    pub fn insert(&mut self, pos: SectionPos, section: ChunkSection);
    pub fn remove(&mut self, pos: &SectionPos) -> Option<ChunkSection>;
    pub fn get(&self, pos: &SectionPos) -> Option<&ChunkSection>;
    pub fn get_mut(&mut self, pos: &SectionPos) -> Option<&mut ChunkSection>;
    pub fn contains(&self, pos: &SectionPos) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
    pub fn total_non_air(&self) -> u64;
}
impl WorldView for SectionCache;
```

## `rustlib::math`

```rust
pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }
impl Vec3 {
    pub const ZERO: Vec3;
    pub const fn new(x: f64, y: f64, z: f64) -> Self;
    pub const fn splat(value: f64) -> Self;
    pub fn length_squared(self) -> f64;
    pub fn length(self) -> f64;
    pub fn dot(self, other: Self) -> f64;
    pub fn cross(self, other: Self) -> Self;
    pub fn normalize_or_zero(self) -> Self;
    pub fn distance_squared(self, other: Self) -> f64;
    pub fn distance(self, other: Self) -> f64;
    pub fn lerp(self, other: Self, t: f64) -> Self;
    pub fn floor(self) -> BlockPos;
}
// + Add, Sub, Mul<f64>

pub struct BlockPos { pub x: i32, pub y: i32, pub z: i32 }
impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self;
    pub const fn offset(self, dx: i32, dy: i32, dz: i32) -> Self;
    pub fn to_vec3(self) -> Vec3;
    pub fn center(self) -> Vec3;
    pub fn distance_squared(self, other: Self) -> f64;
    pub fn neighbours(self) -> [BlockPos; 6];
}

pub struct Aabb { pub min: Vec3, pub max: Vec3 }
impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self;
    pub fn from_center(center: Vec3, half_extent: Vec3) -> Self;
    pub fn block(pos: BlockPos) -> Self;
    pub fn contains(&self, point: Vec3) -> bool;
    pub fn intersects(&self, other: &Aabb) -> bool;
    pub fn expand(&self, amount: f64) -> Self;
    pub fn center(&self) -> Vec3;
    pub fn volume(&self) -> f64;
}
```

## `rustlib::content`

```rust
pub enum ContentError { InvalidLightLevel(u8), InvalidStackSize(u8), ZeroDurability }
pub enum Material { Stone, Dirt, Wood, Metal, Glass, Plant, Misc }   // Default = Stone
pub enum ToolKind { Pickaxe, Axe, Shovel, Hoe, Sword }
pub enum ToolTier { Wood, Stone, Iron, Gold, Diamond, Netherite }
impl ToolTier { pub fn mining_level(self) -> u8; }

pub struct Tool { pub kind: ToolKind, pub tier: ToolTier,
                  pub speed: f32, pub attack_damage: f32 }
impl Tool { pub fn new(kind: ToolKind, tier: ToolTier, speed: f32, attack_damage: f32) -> Self; }

pub struct Block;
impl Block {
    pub fn builder(id: ResourceLocation) -> BlockBuilder;
    pub fn material(&self) -> Material;
    pub fn hardness(&self) -> f32;
    pub fn resistance(&self) -> f32;
    pub fn light(&self) -> u8;
    pub fn is_transparent(&self) -> bool;
    pub fn drops(&self) -> &[ResourceLocation];
}
pub struct BlockBuilder;
impl BlockBuilder {
    pub fn material(self, material: Material) -> Self;
    pub fn hardness(self, hardness: f32) -> Self;
    pub fn resistance(self, resistance: f32) -> Self;
    pub fn light(self, light: u8) -> Self;
    pub fn transparent(self, transparent: bool) -> Self;
    pub fn drops(self, drops: impl IntoIterator<Item = ResourceLocation>) -> Self;
    pub fn build(self) -> Result<Block, ContentError>;
}

pub struct Item;
impl Item {
    pub fn builder(id: ResourceLocation) -> ItemBuilder;
    pub fn max_stack(&self) -> u8;
    pub fn durability(&self) -> Option<u32>;
    pub fn tool(&self) -> Option<Tool>;
    pub fn food_points(&self) -> Option<u8>;
}
pub struct ItemBuilder;
impl ItemBuilder {
    pub fn stack_size(self, max_stack: u8) -> Self;
    pub fn durability(self, durability: u32) -> Self;
    pub fn tool(self, tool: Tool) -> Self;
    pub fn food(self, food_points: u8) -> Self;
    pub fn build(self) -> Result<Item, ContentError>;
}
```

## `rustlib::runtime`

```rust
pub enum LoadError { DuplicateMod(ResourceLocation), Registry(RegistryError) }

pub struct Runtime;
impl Runtime {
    pub fn new(tick_budget: Duration) -> Self;
    pub fn load(&mut self, module: &mut dyn Mod) -> Result<(), LoadError>;
    pub fn finish_loading(&mut self);
    pub fn is_loading(&self) -> bool;
    pub fn register_block(&mut self, block: Block) -> Result<(), RegistryError>;
    pub fn register_item(&mut self, item: Item) -> Result<(), RegistryError>;
    pub fn blocks(&self) -> &Registry<Block>;
    pub fn items(&self) -> &Registry<Item>;
    pub fn events(&self) -> &EventBus;
    pub fn events_mut(&mut self) -> &mut EventBus;
    pub fn scheduler(&self) -> &TickScheduler;
    pub fn scheduler_mut(&mut self) -> &mut TickScheduler;
    pub fn post<E: Event>(&mut self, event: &mut E) -> bool;
    pub fn mods(&self) -> &[ModMetadata];
    pub fn current_tick(&self) -> u64;
    pub fn tick(&mut self) -> TickReport;
    pub fn registries_json(&self) -> String;
}
```

## `rustlib` (racine)

```rust
pub trait Mod: Send {
    fn metadata(&self) -> ModMetadata;
    fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError>;
    fn ready(&mut self, _runtime: &mut Runtime) {}
}
```

## `rustlib_jni`

Le handle est un `*mut Runtime` brut : la JVM en est l'unique propriétaire, et
`destroy` le seul point de libération. Toute la partie `unsafe` du pont tient
dans ce module.

```rust
pub fn create(module: Box<dyn Mod>, tick_budget_nanos: i64) -> Result<jlong, String>;
pub fn destroy(handle: jlong);
pub fn tick(handle: jlong) -> jlong;
pub fn registries_json(env: &mut JNIEnv, handle: jlong) -> jstring;
pub fn on_block_break(env: &mut JNIEnv, handle: jlong, x: jint, y: jint, z: jint,
                      block: &JString, player: &JString) -> jboolean;
pub fn raise(env: &mut JNIEnv, message: &str);

// Génère les symboles JNI fixes attendus par dev.rustlib.RustLib :
//   Java_dev_rustlib_RustLib_nativeCreate
//   Java_dev_rustlib_RustLib_nativeRegistries
//   Java_dev_rustlib_RustLib_nativeTick
//   Java_dev_rustlib_RustLib_nativeBlockBreak
//   Java_dev_rustlib_RustLib_nativeDestroy
export_mod!(|| -> Box<dyn rustlib::Mod> { /* votre mod */ });
```

## `dev.rustlib.RustLib` (Java, dans `rustlib.jar`)

```java
public final class RustLib implements AutoCloseable {
    public static final long DEFAULT_TICK_BUDGET_NANOS = 2_000_000L;

    public static RustLib start();                   // budget par défaut (2 ms)
    public static RustLib start(long budgetNanos);   // 0 => IllegalStateException

    public String  registriesJson();
    public long    tick();                           // tâches exécutées
    public boolean blockBreak(int x, int y, int z, String block, String player);
    @Override public void close();                   // idempotent
}
```

Le natif est toujours chargé depuis le jar, à la première utilisation ; après
`close()`, `registriesJson`, `tick` et `blockBreak` lèvent une
`IllegalStateException` plutôt que d'envoyer un handle libéré à Rust.

→ [Limites et FAQ](10-limites-faq.md)
