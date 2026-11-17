//! Exécution d'un mod : chargement, registres, événements, ticks.
//!
//! Le [`Runtime`] est le point d'entrée unique : il possède les registres, le
//! bus d'événements et l'ordonnanceur, et il est piloté depuis la JVM par le
//! pont [`rustlib-jni`](../../rustlib_jni/index.html) (une méthode Java `tick` →
//! un appel [`Runtime::tick`]).

use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

use crate::content::{Block, Item};
use crate::event::{Event, EventBus};
use crate::id::ResourceLocation;
use crate::manifest::ModMetadata;
use crate::registry::{Registry, RegistryEntry, RegistryError};
use crate::tick::{TickReport, TickScheduler};

/// Erreur de chargement d'un mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Un mod avec le même identifiant est déjà chargé.
    DuplicateMod(ResourceLocation),
    /// Un enregistrement a échoué (doublon d'identifiant, registre gelé).
    Registry(RegistryError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::DuplicateMod(id) => {
                write!(f, "le mod `{id}` est déjà chargé")
            }
            LoadError::Registry(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<RegistryError> for LoadError {
    fn from(value: RegistryError) -> Self {
        LoadError::Registry(value)
    }
}

/// État vivant d'un serveur moddé : registres + bus + ordonnanceur.
///
/// Cycle de vie :
/// 1. [`Runtime::new`]
/// 2. [`Runtime::load`] pour chaque mod (enregistrements)
/// 3. [`Runtime::finish_loading`] (gèle les registres)
/// 4. [`Runtime::tick`] à chaque tick du serveur
pub struct Runtime {
    blocks: Registry<Block>,
    items: Registry<Item>,
    events: EventBus,
    scheduler: TickScheduler,
    mods: Vec<ModMetadata>,
    seen_mods: HashSet<ResourceLocation>,
    tick: u64,
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime")
            .field("mods", &self.mods.len())
            .field("blocks", &self.blocks.len())
            .field("items", &self.items.len())
            .field("tick", &self.tick)
            .finish()
    }
}

impl Runtime {
    /// Crée un runtime avec le budget de temps par tick accordé aux mods.
    pub fn new(tick_budget: Duration) -> Self {
        Self {
            blocks: Registry::new(),
            items: Registry::new(),
            events: EventBus::new(),
            scheduler: TickScheduler::new(tick_budget),
            mods: Vec::new(),
            seen_mods: HashSet::new(),
            tick: 0,
        }
    }

    /// Charge un mod : vérifie son identité puis exécute son enregistrement.
    pub fn load(&mut self, module: &mut dyn crate::Mod) -> Result<(), LoadError> {
        let metadata = module.metadata();
        if !self.seen_mods.insert(metadata.id().clone()) {
            return Err(LoadError::DuplicateMod(metadata.id().clone()));
        }
        self.mods.push(metadata);
        module.register(self)?;
        module.ready(self);
        Ok(())
    }

    /// Termine la phase de chargement : les registres sont gelés et tout ajout
    /// ultérieur échouera.
    pub fn finish_loading(&mut self) {
        self.blocks.freeze();
        self.items.freeze();
    }

    /// Les registres acceptent-ils encore des ajouts ?
    pub fn is_loading(&self) -> bool {
        !self.blocks.is_frozen()
    }

    /// Enregistre un bloc (phase de chargement uniquement).
    pub fn register_block(&mut self, block: Block) -> Result<(), RegistryError> {
        self.blocks.register(block)
    }

    /// Enregistre un item (phase de chargement uniquement).
    pub fn register_item(&mut self, item: Item) -> Result<(), RegistryError> {
        self.items.register(item)
    }

    pub fn blocks(&self) -> &Registry<Block> {
        &self.blocks
    }

    pub fn items(&self) -> &Registry<Item> {
        &self.items
    }

    pub fn events(&self) -> &EventBus {
        &self.events
    }

    /// Accès mutable au bus, pour s'abonner après le chargement.
    pub fn events_mut(&mut self) -> &mut EventBus {
        &mut self.events
    }

    pub fn scheduler(&self) -> &TickScheduler {
        &self.scheduler
    }

    /// Accès mutable à l'ordonnanceur, pour planifier du travail.
    pub fn scheduler_mut(&mut self) -> &mut TickScheduler {
        &mut self.scheduler
    }

    /// Publie un événement et indique s'il peut se produire.
    pub fn post<E: Event>(&mut self, event: &mut E) -> bool {
        self.events.post(event)
    }

    /// Mods chargés, dans l'ordre de chargement.
    pub fn mods(&self) -> &[ModMetadata] {
        &self.mods
    }

    /// Numéro du dernier tick exécuté.
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Exécute un tick : horloge puis tâches planifiées dans la limite du
    /// budget.
    pub fn tick(&mut self) -> TickReport {
        self.tick += 1;
        self.scheduler.run(self.tick)
    }

    /// Sérialise les registres en JSON pour la JVM.
    ///
    /// La JVM crée les blocs et items Minecraft à partir de cette description ;
    /// c'est le seul échange « riche » entre les deux mondes, et il n'a lieu
    /// qu'une fois, au chargement.
    pub fn registries_json(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("{\"mods\":[");
        for (index, meta) in self.mods.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":\"");
            push_escaped(&mut out, &meta.id().to_string());
            out.push_str("\",\"name\":\"");
            push_escaped(&mut out, meta.name());
            out.push_str("\",\"version\":\"");
            push_escaped(&mut out, &meta.version().to_string());
            out.push_str("\"}");
        }

        out.push_str("],\"blocks\":[");
        for (index, block) in self.blocks.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":\"");
            push_escaped(&mut out, &block.id().to_string());
            out.push_str("\",\"material\":\"");
            out.push_str(block.material().as_str());
            out.push_str("\",\"hardness\":");
            out.push_str(&format!("{}", block.hardness()));
            out.push_str(",\"resistance\":");
            out.push_str(&format!("{}", block.resistance()));
            out.push_str(",\"light\":");
            out.push_str(&format!("{}", block.light()));
            out.push_str(",\"transparent\":");
            out.push_str(if block.is_transparent() { "true" } else { "false" });
            out.push_str(",\"drops\":[");
            for (drop_index, drop) in block.drops().iter().enumerate() {
                if drop_index > 0 {
                    out.push(',');
                }
                out.push('"');
                push_escaped(&mut out, &drop.to_string());
                out.push('"');
            }
            out.push_str("]}");
        }

        out.push_str("],\"items\":[");
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":\"");
            push_escaped(&mut out, &item.id().to_string());
            out.push_str("\",\"max_stack\":");
            out.push_str(&format!("{}", item.max_stack()));
            match item.durability() {
                Some(durability) => {
                    out.push_str(",\"durability\":");
                    out.push_str(&format!("{durability}"));
                }
                None => out.push_str(",\"durability\":null"),
            }
            match item.tool() {
                Some(tool) => {
                    out.push_str(",\"tool\":{\"kind\":\"");
                    out.push_str(tool_kind_str(tool.kind));
                    out.push_str("\",\"tier\":");
                    out.push_str(&format!("{}", tool.tier.mining_level()));
                    out.push_str(",\"speed\":");
                    out.push_str(&format!("{}", tool.speed));
                    out.push_str(",\"attack_damage\":");
                    out.push_str(&format!("{}", tool.attack_damage));
                    out.push('}');
                }
                None => out.push_str(",\"tool\":null"),
            }
            match item.food_points() {
                Some(points) => {
                    out.push_str(",\"food_points\":");
                    out.push_str(&format!("{points}"));
                }
                None => out.push_str(",\"food_points\":null"),
            }
            out.push('}');
        }
        out.push_str("]}");
        out
    }
}

fn tool_kind_str(kind: crate::content::ToolKind) -> &'static str {
    use crate::content::ToolKind;
    match kind {
        ToolKind::Pickaxe => "pickaxe",
        ToolKind::Axe => "axe",
        ToolKind::Shovel => "shovel",
        ToolKind::Hoe => "hoe",
        ToolKind::Sword => "sword",
    }
}

/// Écrit une chaîne en échappant les caractères JSON obligatoires.
fn push_escaped(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Item, Material, Tool, ToolKind, ToolTier};
    use crate::event::EventPriority;
    use crate::manifest::Version;
    use crate::Mod;

    struct CopperMod {
        metadata: ModMetadata,
    }

    impl CopperMod {
        fn new() -> Self {
            Self {
                metadata: ModMetadata::builder(
                    ResourceLocation::mc("copper", "copper_expansion"),
                    "Copper Expansion",
                    Version::new(0, 1, 0),
                )
                .author("tests")
                .description("Mod de test")
                .build()
                .unwrap(),
            }
        }
    }

    impl Mod for CopperMod {
        fn metadata(&self) -> ModMetadata {
            self.metadata.clone()
        }

        fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError> {
            runtime.register_block(
                Block::builder(ResourceLocation::mc("copper", "copper_ore"))
                    .material(Material::Stone)
                    .hardness(3.0)
                    .build()
                    .unwrap(),
            )?;
            runtime.register_item(
                Item::builder(ResourceLocation::mc("copper", "copper_pickaxe"))
                    .tool(Tool::new(ToolKind::Pickaxe, ToolTier::Iron, 6.0, 2.0))
                    .build()
                    .unwrap(),
            )?;
            Ok(())
        }
    }

    #[test]
    fn chargement_complet() {
        let mut runtime = Runtime::new(Duration::from_millis(2));
        let mut module = CopperMod::new();
        runtime.load(&mut module).unwrap();
        assert!(runtime.is_loading());
        assert_eq!(runtime.blocks().len(), 1);
        assert_eq!(runtime.items().len(), 1);
        assert_eq!(runtime.mods().len(), 1);

        runtime.finish_loading();
        assert!(!runtime.is_loading());
        assert!(runtime
            .register_item(
                Item::builder(ResourceLocation::mc("copper", "tardif"))
                    .build()
                    .unwrap()
            )
            .is_err());
    }

    #[test]
    fn mod_en_double_refuse() {
        let mut runtime = Runtime::new(Duration::from_millis(2));
        runtime.load(&mut CopperMod::new()).unwrap();
        assert_eq!(
            runtime.load(&mut CopperMod::new()),
            Err(LoadError::DuplicateMod(ResourceLocation::mc(
                "copper",
                "copper_expansion"
            )))
        );
    }

    #[test]
    fn evenements_et_ticks() {
        let mut runtime = Runtime::new(Duration::from_millis(5));
        runtime.events_mut().subscribe::<BlockBreak>(
            EventPriority::Normal,
            |event| {
                if event.block == ResourceLocation::mc("copper", "copper_ore") {
                    event.cancel();
                }
            },
        );

        let mut event = BlockBreak {
            block: ResourceLocation::mc("copper", "copper_ore"),
            cancelled: false,
        };
        assert!(!runtime.post(&mut event));

        let mut ok = BlockBreak {
            block: ResourceLocation::mc("minecraft", "stone"),
            cancelled: false,
        };
        assert!(runtime.post(&mut ok));

        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;
        let ticks = Arc::new(AtomicU64::new(0));
        let t = ticks.clone();
        runtime
            .scheduler_mut()
            .schedule_repeating(1, "test", move |_| {
                t.fetch_add(1, Ordering::Relaxed);
            });
        runtime.tick();
        runtime.tick();
        assert_eq!(runtime.current_tick(), 2);
        assert_eq!(ticks.load(Ordering::Relaxed), 2);
    }

    struct BlockBreak {
        block: ResourceLocation,
        cancelled: bool,
    }

    impl Event for BlockBreak {
        fn is_cancelled(&self) -> bool {
            self.cancelled
        }
        fn cancel(&mut self) {
            self.cancelled = true;
        }
    }

    #[test]
    fn json_des_registres() {
        let mut runtime = Runtime::new(Duration::from_millis(2));
        runtime.load(&mut CopperMod::new()).unwrap();
        let json = runtime.registries_json();
        assert!(json.contains("\"id\":\"copper:copper_ore\""));
        assert!(json.contains("\"material\":\"stone\""));
        assert!(json.contains("\"id\":\"copper:copper_pickaxe\""));
        assert!(json.contains("\"kind\":\"pickaxe\""));
        assert!(json.contains("\"tier\":2"));
        assert!(json.starts_with('{') && json.ends_with('}'));
    }

    #[test]
    fn echappement_json() {
        let mut out = String::new();
        push_escaped(&mut out, "guillemet\" antislash\\ saut\nligne");
        assert_eq!(out, "guillemet\\\" antislash\\\\ saut\\nligne");
    }
}
