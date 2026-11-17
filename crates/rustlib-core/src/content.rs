//! Déclarations de blocs et d'items.
//!
//! Un mod RustLib décrit son contenu de façon déclarative ; le pont JNI
//! transmet ensuite ces descriptions au registre Java, qui crée les objets
//! Minecraft correspondants.

use std::fmt;

use crate::id::ResourceLocation;
use crate::registry::RegistryEntry;

/// Erreur lors de la construction d'un bloc ou d'un item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentError {
    /// Le niveau de lumière doit tenir sur 4 bits (`0..=15`).
    InvalidLightLevel(u8),
    /// La taille de pile doit être comprise entre 1 et 64.
    InvalidStackSize(u8),
    /// Une durabilité nulle n'a pas de sens (`None` = indestructible).
    ZeroDurability,
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentError::InvalidLightLevel(level) => {
                write!(f, "niveau de lumière {level} invalide (attendu 0..=15)")
            }
            ContentError::InvalidStackSize(size) => {
                write!(f, "taille de pile {size} invalide (attendu 1..=64)")
            }
            ContentError::ZeroDurability => {
                write!(f, "durabilité nulle : utiliser None pour un item incassable")
            }
        }
    }
}

impl std::error::Error for ContentError {}

/// Matériau d'un bloc, utilisé par Minecraft pour les sons et les outils.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Material {
    #[default]
    Stone,
    Dirt,
    Wood,
    Metal,
    Glass,
    Plant,
    Misc,
}

impl Material {
    /// Nom minuscule utilisé dans les échanges avec la JVM.
    pub fn as_str(self) -> &'static str {
        match self {
            Material::Stone => "stone",
            Material::Dirt => "dirt",
            Material::Wood => "wood",
            Material::Metal => "metal",
            Material::Glass => "glass",
            Material::Plant => "plant",
            Material::Misc => "misc",
        }
    }
}

/// Définition d'un bloc.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    id: ResourceLocation,
    material: Material,
    hardness: f32,
    resistance: f32,
    light: u8,
    transparent: bool,
    drops: Vec<ResourceLocation>,
}

impl Block {
    /// Commence la construction d'un bloc identifié par `id`.
    pub fn builder(id: ResourceLocation) -> BlockBuilder {
        BlockBuilder {
            block: Block {
                id,
                material: Material::Stone,
                hardness: 1.0,
                resistance: 1.0,
                light: 0,
                transparent: false,
                drops: Vec::new(),
            },
        }
    }

    pub fn material(&self) -> Material {
        self.material
    }

    pub fn hardness(&self) -> f32 {
        self.hardness
    }

    pub fn resistance(&self) -> f32 {
        self.resistance
    }

    /// Niveau de lumière émis (`0..=15`).
    pub fn light(&self) -> u8 {
        self.light
    }

    pub fn is_transparent(&self) -> bool {
        self.transparent
    }

    /// Items lâchés à la casse (vide = le bloc lui-même).
    pub fn drops(&self) -> &[ResourceLocation] {
        &self.drops
    }
}

impl RegistryEntry for Block {
    fn id(&self) -> &ResourceLocation {
        &self.id
    }
}

/// Constructeur de [`Block`].
#[derive(Debug)]
pub struct BlockBuilder {
    block: Block,
}

impl BlockBuilder {
    pub fn material(mut self, material: Material) -> Self {
        self.block.material = material;
        self
    }

    /// Temps de minage de base (pierre = 1.5, obsidienne = 50.0).
    pub fn hardness(mut self, hardness: f32) -> Self {
        self.block.hardness = hardness;
        self
    }

    /// Résistance aux explosions.
    pub fn resistance(mut self, resistance: f32) -> Self {
        self.block.resistance = resistance;
        self
    }

    /// Niveau de lumière émis, validé à la construction.
    pub fn light(mut self, light: u8) -> Self {
        self.block.light = light;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.block.transparent = transparent;
        self
    }

    /// Items lâchés à la casse.
    pub fn drops(mut self, drops: impl IntoIterator<Item = ResourceLocation>) -> Self {
        self.block.drops = drops.into_iter().collect();
        self
    }

    pub fn build(self) -> Result<Block, ContentError> {
        if self.block.light > 15 {
            return Err(ContentError::InvalidLightLevel(self.block.light));
        }
        Ok(self.block)
    }
}

/// Type d'outil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Sword,
}

/// Niveau de matériau d'un outil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTier {
    Wood,
    Stone,
    Iron,
    Gold,
    Diamond,
    Netherite,
}

impl ToolTier {
    /// Niveau de minage (0 = bois, 4 = netherite), utilisé par Minecraft pour
    /// décider quels blocs l'outil peut casser.
    pub fn mining_level(self) -> u8 {
        match self {
            ToolTier::Wood | ToolTier::Gold => 0,
            ToolTier::Stone => 1,
            ToolTier::Iron => 2,
            ToolTier::Diamond => 3,
            ToolTier::Netherite => 4,
        }
    }
}

/// Caractéristiques d'un outil.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tool {
    pub kind: ToolKind,
    pub tier: ToolTier,
    /// Multiplicateur de vitesse de minage.
    pub speed: f32,
    /// Dégâts supplémentaires à l'attaque.
    pub attack_damage: f32,
}

impl Tool {
    pub fn new(kind: ToolKind, tier: ToolTier, speed: f32, attack_damage: f32) -> Self {
        Self {
            kind,
            tier,
            speed,
            attack_damage,
        }
    }
}

/// Définition d'un item.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    id: ResourceLocation,
    max_stack: u8,
    durability: Option<u32>,
    tool: Option<Tool>,
    food_points: Option<u8>,
}

impl Item {
    /// Commence la construction d'un item identifié par `id`.
    pub fn builder(id: ResourceLocation) -> ItemBuilder {
        ItemBuilder {
            item: Item {
                id,
                max_stack: 64,
                durability: None,
                tool: None,
                food_points: None,
            },
        }
    }

    pub fn max_stack(&self) -> u8 {
        self.max_stack
    }

    /// `None` = item empilable non destructible.
    pub fn durability(&self) -> Option<u32> {
        self.durability
    }

    pub fn tool(&self) -> Option<Tool> {
        self.tool
    }

    /// Points de nourriture rendus (par unité).
    pub fn food_points(&self) -> Option<u8> {
        self.food_points
    }
}

impl RegistryEntry for Item {
    fn id(&self) -> &ResourceLocation {
        &self.id
    }
}

/// Constructeur de [`Item`].
#[derive(Debug)]
pub struct ItemBuilder {
    item: Item,
}

impl ItemBuilder {
    /// Taille de pile maximale (1 pour les outils).
    pub fn stack_size(mut self, max_stack: u8) -> Self {
        self.item.max_stack = max_stack;
        self
    }

    /// Durabilité de l'item (implique une pile de 1).
    pub fn durability(mut self, durability: u32) -> Self {
        self.item.durability = Some(durability);
        self.item.max_stack = 1;
        self
    }

    pub fn tool(mut self, tool: Tool) -> Self {
        self.item.tool = Some(tool);
        self.item.max_stack = 1;
        self
    }

    pub fn food(mut self, food_points: u8) -> Self {
        self.item.food_points = Some(food_points);
        self
    }

    pub fn build(self) -> Result<Item, ContentError> {
        if !(1..=64).contains(&self.item.max_stack) {
            return Err(ContentError::InvalidStackSize(self.item.max_stack));
        }
        if self.item.durability == Some(0) {
            return Err(ContentError::ZeroDurability);
        }
        Ok(self.item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloc_par_defaut() {
        let block = Block::builder(ResourceLocation::mc("copper", "copper_ore"))
            .build()
            .unwrap();
        assert_eq!(block.material(), Material::Stone);
        assert_eq!(block.hardness(), 1.0);
        assert_eq!(block.light(), 0);
        assert!(!block.is_transparent());
        assert_eq!(block.id().path(), "copper_ore");
    }

    #[test]
    fn bloc_parametre() {
        let block = Block::builder(ResourceLocation::mc("copper", "lamp"))
            .material(Material::Metal)
            .hardness(3.5)
            .resistance(6.0)
            .light(15)
            .transparent(true)
            .drops([ResourceLocation::mc("copper", "ingot")])
            .build()
            .unwrap();
        assert_eq!(block.light(), 15);
        assert!(block.is_transparent());
        assert_eq!(block.drops(), [ResourceLocation::mc("copper", "ingot")]);
    }

    #[test]
    fn lumiere_invalide() {
        assert_eq!(
            Block::builder(ResourceLocation::mc("copper", "lamp"))
                .light(16)
                .build(),
            Err(ContentError::InvalidLightLevel(16))
        );
    }

    #[test]
    fn item_outil() {
        let item = Item::builder(ResourceLocation::mc("copper", "sword"))
            .tool(Tool::new(ToolKind::Sword, ToolTier::Iron, 6.0, 3.0))
            .build()
            .unwrap();
        assert_eq!(item.max_stack(), 1);
        assert_eq!(item.durability(), None);
        assert_eq!(item.tool().unwrap().tier.mining_level(), 2);
    }

    #[test]
    fn item_empilable_et_nourriture() {
        let item = Item::builder(ResourceLocation::mc("copper", "apple"))
            .stack_size(16)
            .food(4)
            .build()
            .unwrap();
        assert_eq!(item.max_stack(), 16);
        assert_eq!(item.food_points(), Some(4));
    }

    #[test]
    fn erreurs_d_item() {
        assert_eq!(
            Item::builder(ResourceLocation::mc("copper", "x"))
                .stack_size(0)
                .build(),
            Err(ContentError::InvalidStackSize(0))
        );
        assert_eq!(
            Item::builder(ResourceLocation::mc("copper", "x"))
                .stack_size(65)
                .build(),
            Err(ContentError::InvalidStackSize(65))
        );
        assert_eq!(
            Item::builder(ResourceLocation::mc("copper", "x"))
                .durability(0)
                .build(),
            Err(ContentError::ZeroDurability)
        );
    }

    #[test]
    fn durabilite_force_une_pile() {
        let item = Item::builder(ResourceLocation::mc("copper", "pickaxe"))
            .durability(250)
            .build()
            .unwrap();
        assert_eq!(item.max_stack(), 1);
        assert_eq!(item.durability(), Some(250));
    }
}
