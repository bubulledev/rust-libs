//! Registre typé : une collection d'entrées indexées par identifiant, qui se
//! gèle une fois le chargement des mods terminé.
//!
//! Le cycle de vie est le même que celui des registres Minecraft : pendant le
//! chargement on peut enregistrer, après quoi le registre est gelé et toute
//! tentative d'ajout échoue (au lieu de produire un monde désynchronisé).

use std::collections::HashMap;
use std::fmt;

use crate::id::ResourceLocation;

/// Élément identifiable pouvant vivre dans un [`Registry`].
pub trait RegistryEntry {
    fn id(&self) -> &ResourceLocation;
}

/// Erreur d'enregistrement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Un élément porte déjà cet identifiant.
    Duplicate(ResourceLocation),
    /// Le registre a été gelé (chargement terminé).
    Frozen,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Duplicate(id) => write!(
                f,
                "l'identifiant `{id}` est déjà enregistré (conflit entre deux mods ?)"
            ),
            RegistryError::Frozen => write!(
                f,
                "le registre est gelé : les enregistrements doivent avoir lieu pendant le chargement"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Registre typé conservant l'ordre d'enregistrement.
#[derive(Debug)]
pub struct Registry<T> {
    entries: Vec<T>,
    index: HashMap<ResourceLocation, usize>,
    frozen: bool,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
            frozen: false,
        }
    }

    /// Nombre d'entrées enregistrées.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Le registre accepte-t-il encore des ajouts ?
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Gèle le registre : plus aucun ajout n'est possible ensuite.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Itère les entrées dans l'ordre d'enregistrement.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter()
    }
}

impl<T: RegistryEntry> Registry<T> {
    /// Ajoute une entrée en vérifiant l'unicité de son identifiant.
    pub fn register(&mut self, entry: T) -> Result<(), RegistryError> {
        if self.frozen {
            return Err(RegistryError::Frozen);
        }
        let id = entry.id().clone();
        if self.index.contains_key(&id) {
            return Err(RegistryError::Duplicate(id));
        }
        self.index.insert(id, self.entries.len());
        self.entries.push(entry);
        Ok(())
    }

    /// Recherche par identifiant.
    pub fn get(&self, id: &ResourceLocation) -> Option<&T> {
        self.index.get(id).map(|&i| &self.entries[i])
    }

    pub fn contains(&self, id: &ResourceLocation) -> bool {
        self.index.contains_key(id)
    }

    /// Itère les identifiants dans l'ordre d'enregistrement.
    pub fn ids(&self) -> impl Iterator<Item = &ResourceLocation> {
        self.entries.iter().map(|e| e.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Dummy(ResourceLocation);

    impl RegistryEntry for Dummy {
        fn id(&self) -> &ResourceLocation {
            &self.0
        }
    }

    fn dummy(path: &str) -> Dummy {
        Dummy(ResourceLocation::mc("test", path))
    }

    #[test]
    fn enregistrement_et_lookup() {
        let mut registry = Registry::new();
        registry.register(dummy("a")).unwrap();
        registry.register(dummy("b")).unwrap();
        assert_eq!(registry.len(), 2);
        let id = ResourceLocation::mc("test", "b");
        assert_eq!(registry.get(&id), Some(&dummy("b")));
        assert!(registry.contains(&id));
        assert!(registry.get(&ResourceLocation::mc("test", "c")).is_none());
    }

    #[test]
    fn ordre_d_enregistrement_conserve() {
        let mut registry = Registry::new();
        for path in ["z", "a", "m"] {
            registry.register(dummy(path)).unwrap();
        }
        let ids: Vec<_> = registry.ids().map(|id| id.path()).collect();
        assert_eq!(ids, ["z", "a", "m"]);
    }

    #[test]
    fn doublon_refuse() {
        let mut registry = Registry::new();
        registry.register(dummy("a")).unwrap();
        assert_eq!(
            registry.register(dummy("a")),
            Err(RegistryError::Duplicate(ResourceLocation::mc("test", "a")))
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn gel_du_registre() {
        let mut registry = Registry::new();
        registry.register(dummy("a")).unwrap();
        registry.freeze();
        assert!(registry.is_frozen());
        assert_eq!(registry.register(dummy("b")), Err(RegistryError::Frozen));
        assert_eq!(registry.len(), 1);
    }
}
