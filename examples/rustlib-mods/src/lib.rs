//! # RustLib Mods
//!
//! Mod d'exemple complet : deux blocs, deux items, un gestionnaire d'événement
//! et une tâche planifiée.
//!
//! Le fichier se lit en deux parties :
//!
//! 1. `RustLibMods`, du Rust pur qui ne dépend pas de la JVM (testable avec
//!    `cargo test`) ;
//! 2. un unique appel à `rustlib_jni::export_mod!`, qui exporte les symboles
//!    JNI attendus par la classe Java `dev.rustlib.RustLib` du jar `rustlib.jar`.
//!
//! La logique métier ne doit jamais vivre dans le pont JNI : gardez le pont
//! mince, c'est le seul endroit du mod qui ne peut pas être testé sans JVM
//! (et il est fourni par la macro).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rustlib::events::BlockBreak;
use rustlib::prelude::*;

/// Namespace réservé à ce mod.
pub const NAMESPACE: &str = "copper";

/// Quota de minerai cassé par fenêtre de 100 ticks.
const ORE_QUOTA_PER_WINDOW: u64 = 64;

/// Durée d'une fenêtre de statistiques, en ticks.
const WINDOW_TICKS: u64 = 100;

/// Compteurs partagés entre la tâche planifiée et le gestionnaire d'événement.
#[derive(Debug, Default)]
struct Stats {
    ore_breaks: AtomicU64,
    last_window: AtomicU64,
    blocked_breaks: AtomicU64,
}

/// Le mod.
#[derive(Debug)]
pub struct RustLibMods {
    meta: ModMetadata,
    stats: Arc<Stats>,
}

impl RustLibMods {
    pub fn new() -> Self {
        Self {
            meta: ModMetadata::builder(
                ResourceLocation::mc(NAMESPACE, "rustlib_mods"),
                "RustLib Mods",
                Version::new(0, 1, 0),
            )
            .author("exemple RustLib")
            .description("Blocs et items de cuivre en Rust + limite de minage intensif.")
            .license("MIT")
            .minecraft_versions(">=1.21 <1.22")
            .build()
            .expect("métadonnées valides"),
            stats: Arc::new(Stats::default()),
        }
    }

    /// Nombre de casses de minerai observées pendant la dernière fenêtre.
    pub fn last_window(&self) -> u64 {
        self.stats.last_window.load(Ordering::Relaxed)
    }

    /// Nombre de casses annulées par le quota.
    pub fn blocked_breaks(&self) -> u64 {
        self.stats.blocked_breaks.load(Ordering::Relaxed)
    }
}

impl Default for RustLibMods {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for RustLibMods {
    fn metadata(&self) -> ModMetadata {
        self.meta.clone()
    }

    fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError> {
        runtime.register_block(
            Block::builder(ResourceLocation::mc(NAMESPACE, "copper_ore"))
                .material(Material::Stone)
                .hardness(3.0)
                .resistance(3.0)
                .drops([ResourceLocation::mc(NAMESPACE, "raw_copper")])
                .build()
                .expect("bloc valide"),
        )?;

        runtime.register_block(
            Block::builder(ResourceLocation::mc(NAMESPACE, "copper_lamp"))
                .material(Material::Metal)
                .hardness(3.0)
                .light(15)
                .build()
                .expect("bloc valide"),
        )?;

        runtime.register_item(
            Item::builder(ResourceLocation::mc(NAMESPACE, "raw_copper"))
                .stack_size(64)
                .build()
                .expect("item valide"),
        )?;

        runtime.register_item(
            Item::builder(ResourceLocation::mc(NAMESPACE, "copper_pickaxe"))
                .tool(Tool::new(ToolKind::Pickaxe, ToolTier::Stone, 5.0, 1.0))
                .durability(190)
                .build()
                .expect("item valide"),
        )?;

        let stats = self.stats.clone();
        runtime
            .events_mut()
            .subscribe::<BlockBreak>(EventPriority::Normal, move |event| {
                if event.block != ResourceLocation::mc(NAMESPACE, "copper_ore") {
                    return;
                }
                let count = stats.ore_breaks.fetch_add(1, Ordering::Relaxed) + 1;
                if count > ORE_QUOTA_PER_WINDOW {
                    stats.blocked_breaks.fetch_add(1, Ordering::Relaxed);
                    event.cancel();
                }
            });

        Ok(())
    }

    fn ready(&mut self, runtime: &mut Runtime) {
        let stats = self.stats.clone();
        runtime
            .scheduler_mut()
            .schedule_repeating(WINDOW_TICKS, "copper_stats_window", move |_| {
                let window = stats.ore_breaks.swap(0, Ordering::Relaxed);
                stats.last_window.store(window, Ordering::Relaxed);
            });
    }
}

// Les cinq symboles JNI (`Java_dev_rustlib_RustLib_*`) sont générés ici :
// ils relient la classe Java `dev.rustlib.RustLib` du jar `rustlib.jar` à ce mod.
rustlib_jni::export_mod!(|| Box::new(RustLibMods::new()));

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn enregistrement_du_contenu() {
        let mut runtime = Runtime::new(Duration::from_millis(2));
        let mut module = RustLibMods::new();
        runtime.load(&mut module).unwrap();
        runtime.finish_loading();

        assert_eq!(runtime.blocks().len(), 2);
        assert_eq!(runtime.items().len(), 2);
        let lamp = runtime
            .blocks()
            .get(&ResourceLocation::mc(NAMESPACE, "copper_lamp"))
            .unwrap();
        assert_eq!(lamp.light(), 15);
        let json = runtime.registries_json();
        assert!(json.contains("copper:copper_ore"));
    }

    #[test]
    fn quota_de_minage() {
        let mut runtime = Runtime::new(Duration::from_millis(5));
        let mut module = RustLibMods::new();
        runtime.load(&mut module).unwrap();
        runtime.finish_loading();

        let ore = ResourceLocation::mc(NAMESPACE, "copper_ore");
        let mut allowed = 0;
        let mut refused = 0;
        for _ in 0..(ORE_QUOTA_PER_WINDOW + 10) {
            let mut event = BlockBreak::new(ore.clone(), BlockPos::new(0, 0, 0), "joueur");
            if runtime.post(&mut event) {
                allowed += 1;
            } else {
                refused += 1;
            }
        }
        assert_eq!(allowed, ORE_QUOTA_PER_WINDOW);
        assert_eq!(refused, 10);
        assert_eq!(module.blocked_breaks(), 10);

        // Les blocs hors quota ne sont pas concernés.
        let mut stone = BlockBreak::new(
            ResourceLocation::parse("minecraft:stone").unwrap(),
            BlockPos::new(0, 0, 0),
            "joueur",
        );
        assert!(runtime.post(&mut stone));
    }

    #[test]
    fn fenetre_remise_a_zero_par_la_tache() {
        let mut runtime = Runtime::new(Duration::from_millis(5));
        let mut module = RustLibMods::new();
        runtime.load(&mut module).unwrap();
        runtime.finish_loading();

        let ore = ResourceLocation::mc(NAMESPACE, "copper_ore");
        for _ in 0..5 {
            let mut event = BlockBreak::new(ore.clone(), BlockPos::new(0, 0, 0), "joueur");
            runtime.post(&mut event);
        }
        for _ in 0..WINDOW_TICKS {
            runtime.tick();
        }
        assert_eq!(module.last_window(), 5);

        // Le quota repart de zéro pour la fenêtre suivante.
        let mut event = BlockBreak::new(ore, BlockPos::new(0, 0, 0), "joueur");
        assert!(runtime.post(&mut event));
    }
}
