//! Accès aux blocs côté Rust : sections de chunk denses et cache de sections.
//!
//! C'est ici que se trouve le gain de performance réel de RustLib : au lieu de
//! franchir la frontière JNI pour chaque lecture de bloc (ce qui coûte plus
//! cher qu'un accès Java), le pont copie les sections de chunk utiles une fois,
//! puis tous les calculs se font en Rust sur des tableaux contigus.
//!
//! Une section fait 16×16×16 = 4096 états de blocs en `u16` (8 Kio), soit un
//! accès O(1) par arithmétique de bits.

use std::collections::HashMap;

use crate::math::BlockPos;

/// Identifiant numérique d'un état de bloc (`BlockState` côté Minecraft).
pub type BlockStateId = u16;

/// L'air est toujours l'état 0 dans le registre Minecraft.
pub const AIR: BlockStateId = 0;

/// Taille d'une section de chunk sur chaque axe.
pub const SECTION_SIZE: usize = 16;

/// Nombre d'états de blocs dans une section.
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE;

/// Index linéaire d'un bloc dans une section (`x`/`z` varient le plus vite).
#[inline]
pub const fn section_index(x: usize, y: usize, z: usize) -> usize {
    (y << 8) | (z << 4) | x
}

/// Coordonnées d'une section dans le monde (chunk X, section Y, chunk Z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SectionPos {
    pub chunk_x: i32,
    pub section_y: i32,
    pub chunk_z: i32,
}

impl SectionPos {
    pub const fn new(chunk_x: i32, section_y: i32, chunk_z: i32) -> Self {
        Self {
            chunk_x,
            section_y,
            chunk_z,
        }
    }

    /// Section contenant une position de bloc.
    ///
    /// `>> 4` arrondit vers le bas, y compris pour les coordonnées négatives,
    /// ce qui correspond au découpage de Minecraft.
    pub fn from_block(pos: BlockPos) -> Self {
        Self {
            chunk_x: pos.x >> 4,
            section_y: pos.y >> 4,
            chunk_z: pos.z >> 4,
        }
    }
}

/// Section de chunk dense.
#[derive(Clone)]
pub struct ChunkSection {
    states: Box<[BlockStateId; SECTION_VOLUME]>,
    non_air: u32,
}

impl Default for ChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ChunkSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkSection")
            .field("non_air", &self.non_air)
            .finish()
    }
}

impl ChunkSection {
    /// Section vide (uniquement de l'air).
    pub fn new() -> Self {
        Self {
            states: Box::new([AIR; SECTION_VOLUME]),
            non_air: 0,
        }
    }

    /// Construit une section à partir de ses 4096 états (ordre
    /// [`section_index`]).
    ///
    /// # Panics
    /// Si `states` ne contient pas exactement 4096 états.
    pub fn from_states(states: &[BlockStateId]) -> Self {
        assert_eq!(
            states.len(),
            SECTION_VOLUME,
            "une section doit contenir exactement {SECTION_VOLUME} états"
        );
        let mut section = Self::new();
        for (index, &state) in states.iter().enumerate() {
            section.states[index] = state;
            if state != AIR {
                section.non_air += 1;
            }
        }
        section
    }

    /// Lit l'état d'un bloc, en coordonnées locales `0..16`.
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockStateId {
        self.states[section_index(x, y, z)]
    }

    /// Écrit l'état d'un bloc, en coordonnées locales `0..16`.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, state: BlockStateId) {
        let index = section_index(x, y, z);
        let previous = self.states[index];
        if previous == state {
            return;
        }
        match (previous == AIR, state == AIR) {
            (true, false) => self.non_air += 1,
            (false, true) => self.non_air -= 1,
            _ => {}
        }
        self.states[index] = state;
    }

    /// Nombre de blocs non vides (accès O(1), maintenu à l'écriture).
    pub fn non_air_count(&self) -> u32 {
        self.non_air
    }

    /// Section entièrement vide ?
    pub fn is_empty(&self) -> bool {
        self.non_air == 0
    }

    /// Remplit la section avec un état unique.
    pub fn fill(&mut self, state: BlockStateId) {
        for slot in self.states.iter_mut() {
            *slot = state;
        }
        self.non_air = if state == AIR { 0 } else { SECTION_VOLUME as u32 };
    }

    /// Vue brute des états, utile pour envoyer la section à la JVM.
    pub fn as_states(&self) -> &[BlockStateId; SECTION_VOLUME] {
        &self.states
    }
}

/// Vue en lecture sur un monde.
pub trait WorldView {
    /// État du bloc à cette position, ou `None` si le chunk n'est pas chargé.
    fn block_state_at(&self, pos: BlockPos) -> Option<BlockStateId>;

    /// Le chunk contenant cette position est-il chargé ?
    fn is_loaded(&self, pos: BlockPos) -> bool {
        self.block_state_at(pos).is_some()
    }
}

/// Cache de sections de chunks maintenu côté Rust.
///
/// Le pont JNI insère les sections reçues de la JVM, puis les mods lisent
/// gratuitement pendant tout le temps où le chunk reste chargé.
#[derive(Debug, Default)]
pub struct SectionCache {
    sections: HashMap<SectionPos, ChunkSection>,
}

impl SectionCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insère ou remplace une section.
    pub fn insert(&mut self, pos: SectionPos, section: ChunkSection) {
        self.sections.insert(pos, section);
    }

    pub fn remove(&mut self, pos: &SectionPos) -> Option<ChunkSection> {
        self.sections.remove(pos)
    }

    pub fn get(&self, pos: &SectionPos) -> Option<&ChunkSection> {
        self.sections.get(pos)
    }

    pub fn get_mut(&mut self, pos: &SectionPos) -> Option<&mut ChunkSection> {
        self.sections.get_mut(pos)
    }

    pub fn contains(&self, pos: &SectionPos) -> bool {
        self.sections.contains_key(pos)
    }

    /// Nombre de sections en cache.
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Vide le cache (déchargement de monde, changement de dimension).
    pub fn clear(&mut self) {
        self.sections.clear();
    }

    /// Nombre de blocs non vides de toutes les sections en cache.
    pub fn total_non_air(&self) -> u64 {
        self.sections
            .values()
            .map(|section| u64::from(section.non_air_count()))
            .sum()
    }
}

impl WorldView for SectionCache {
    fn block_state_at(&self, pos: BlockPos) -> Option<BlockStateId> {
        let section = self.sections.get(&SectionPos::from_block(pos))?;
        Some(section.get(
            pos.x.rem_euclid(SECTION_SIZE as i32) as usize,
            pos.y.rem_euclid(SECTION_SIZE as i32) as usize,
            pos.z.rem_euclid(SECTION_SIZE as i32) as usize,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_unique_et_contigu() {
        let mut seen = vec![false; SECTION_VOLUME];
        for y in 0..SECTION_SIZE {
            for z in 0..SECTION_SIZE {
                for x in 0..SECTION_SIZE {
                    let index = section_index(x, y, z);
                    assert!(index < SECTION_VOLUME);
                    assert!(!seen[index], "index {index} dupliqué");
                    seen[index] = true;
                }
            }
        }
        assert!(seen.into_iter().all(|v| v));
        assert_eq!(section_index(0, 0, 0), 0);
        assert_eq!(section_index(15, 15, 15), SECTION_VOLUME - 1);
    }

    #[test]
    fn lecture_ecriture_et_compteur() {
        let mut section = ChunkSection::new();
        assert!(section.is_empty());
        assert_eq!(section.get(1, 2, 3), AIR);

        section.set(1, 2, 3, 7);
        assert_eq!(section.get(1, 2, 3), 7);
        assert_eq!(section.non_air_count(), 1);

        // Réécrire le même état ne change pas le compteur.
        section.set(1, 2, 3, 7);
        assert_eq!(section.non_air_count(), 1);

        section.set(1, 2, 3, AIR);
        assert_eq!(section.non_air_count(), 0);
        assert!(section.is_empty());
    }

    #[test]
    fn remplissage_et_etats() {
        let mut section = ChunkSection::new();
        section.fill(42);
        assert_eq!(section.non_air_count(), SECTION_VOLUME as u32);
        assert!(section.as_states().iter().all(|&s| s == 42));

        let copy = ChunkSection::from_states(section.as_states());
        assert_eq!(copy.non_air_count(), SECTION_VOLUME as u32);
    }

    #[test]
    fn positions_de_section_negatives() {
        assert_eq!(
            SectionPos::from_block(BlockPos::new(-1, -1, -1)),
            SectionPos::new(-1, -1, -1)
        );
        assert_eq!(
            SectionPos::from_block(BlockPos::new(15, 31, 16)),
            SectionPos::new(0, 1, 1)
        );
    }

    #[test]
    fn cache_et_acces_monde() {
        let mut cache = SectionCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.block_state_at(BlockPos::new(0, 0, 0)), None);

        let mut section = ChunkSection::new();
        section.set(3, 4, 5, 11);
        let pos = SectionPos::new(0, 0, 0);
        cache.insert(pos, section);

        assert_eq!(
            cache.block_state_at(BlockPos::new(3, 4, 5)),
            Some(11),
            "coordonnées locales"
        );
        assert_eq!(cache.block_state_at(BlockPos::new(19, 4, 5)), None);
        assert_eq!(cache.total_non_air(), 1);

        // Coordonnées négatives : le modulo euclidien doit retomber dans 0..16.
        let mut negative = ChunkSection::new();
        negative.set(15, 15, 15, 3);
        cache.insert(SectionPos::new(-1, -1, -1), negative);
        assert_eq!(
            cache.block_state_at(BlockPos::new(-1, -1, -1)),
            Some(3)
        );

        cache.clear();
        assert!(cache.is_empty());
    }
}
