//! # RustLib
//!
//! Bibliothèque Rust pour écrire des mods Minecraft dont les parties coûteuses
//! tournent hors de la JVM.
//!
//! RustLib ne remplace pas Minecraft : il fournit la couche Rust d'un mod dont
//! l'entrée Java reste un mod Fabric classique. La JVM garde le rendu, le
//! réseau et la logique vanilla ; le Rust prend en charge ce qui gagne
//! réellement à être natif :
//!
//! * les **registres** de blocs et d'items, décrits une fois en Rust puis
//!   envoyés à la JVM au chargement ([`runtime::Runtime::registries_json`]) ;
//! * les **événements** (casse de bloc, tick, connexion…) avec priorité et
//!   annulation ([`event`]) ;
//! * le **travail lourd borné dans le temps** grâce à un budget par tick
//!   ([`tick::TickScheduler`]) ;
//! * les **calculs spatiaux sur les chunks**, faits sur des tableaux contigus
//!   côté Rust ([`world::ChunkSection`]).
//!
//! ## Démarrage
//!
//! ```
//! use rustlib::prelude::*;
//! use std::time::Duration;
//!
//! struct MonMod {
//!     meta: ModMetadata,
//! }
//!
//! impl MonMod {
//!     fn new() -> Self {
//!         Self {
//!             meta: ModMetadata::builder(
//!                 ResourceLocation::mc("monmod", "mon_mod"),
//!                 "Mon Mod",
//!                 Version::new(0, 1, 0),
//!             )
//!             .author("moi")
//!             .description("Exemple minimal.")
//!             .build()
//!             .unwrap(),
//!         }
//!     }
//! }
//!
//! impl Mod for MonMod {
//!     fn metadata(&self) -> ModMetadata {
//!         self.meta.clone()
//!     }
//!
//!     fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError> {
//!         runtime.register_block(
//!             Block::builder(ResourceLocation::mc("monmod", "bloc_bizarre"))
//!                 .hardness(2.0)
//!                 .build()
//!                 .unwrap(),
//!         )?;
//!         Ok(())
//!     }
//! }
//!
//! let mut runtime = Runtime::new(Duration::from_millis(2));
//! runtime.load(&mut MonMod::new()).unwrap();
//! runtime.finish_loading();
//! assert_eq!(runtime.blocks().len(), 1);
//! ```
//!
//! ## Garanties
//!
//! * **Aucune dépendance** : le crate compile seul, sans `serde`, sans
//!   `glam`, sans runtime asynchrone.
//! * **Aucune allocation dans le tick** : les noms de tâches sont des `&'static
//!   str`, les sections de chunk sont des tableaux pré-alloués.
//! * **Erreurs explicites** : identifiants validés, doublons refusés,
//!   registres gelés après chargement.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod content;
pub mod event;
pub mod events;
pub mod id;
pub mod manifest;
pub mod math;
pub mod registry;
pub mod runtime;
pub mod tick;
pub mod world;

pub use content::{Block, Item, Material, Tool, ToolKind, ToolTier};
pub use event::{Event, EventBus, EventPriority, HandlerId};
pub use events::{BlockBreak, BlockPlace, PlayerDamage, PlayerJoin, PlayerQuit, ServerTick};
pub use id::ResourceLocation;
pub use manifest::{ModMetadata, Version};
pub use math::{Aabb, BlockPos, Vec3};
pub use registry::{Registry, RegistryEntry, RegistryError};
pub use runtime::{LoadError, Runtime};
pub use tick::{TickContext, TickReport, TickScheduler};
pub use world::{BlockStateId, ChunkSection, SectionCache, SectionPos, WorldView};

/// Un mod RustLib.
///
/// Le runtime appelle [`Mod::register`] une seule fois pendant le chargement,
/// puis [`Mod::ready`] quand tous les mods sont enregistrés. Les hooks de tick
/// passent par [`tick::TickScheduler`], ce qui permet de borner le temps passé
/// dans votre code.
pub trait Mod: Send {
    /// Métadonnées (identité, version, auteurs).
    fn metadata(&self) -> ModMetadata;

    /// Phase d'enregistrement : blocs, items, abonnements aux événements.
    fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError>;

    /// Appelé après l'enregistrement de tous les mods, avant le gel des
    /// registres. Par défaut ne fait rien.
    fn ready(&mut self, _runtime: &mut Runtime) {}
}

/// Ré-exports des types les plus utilisés.
pub mod prelude {
    pub use crate::content::{Block, Item, Material, Tool, ToolKind, ToolTier};
    pub use crate::event::{Event, EventBus, EventPriority, HandlerId};
    pub use crate::events::{BlockBreak, BlockPlace, PlayerDamage, PlayerJoin, PlayerQuit, ServerTick};
    pub use crate::id::ResourceLocation;
    pub use crate::manifest::{ModMetadata, Version};
    pub use crate::math::{Aabb, BlockPos, Vec3};
    pub use crate::registry::{Registry, RegistryEntry, RegistryError};
    pub use crate::runtime::{LoadError, Runtime};
    pub use crate::tick::{TickContext, TickReport, TickScheduler};
    pub use crate::world::{BlockStateId, ChunkSection, SectionCache, SectionPos, WorldView};
    pub use crate::Mod;
}
