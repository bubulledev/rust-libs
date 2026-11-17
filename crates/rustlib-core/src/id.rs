//! Identifiants namespacés (`namespace:path`), le format utilisé partout dans
//! Minecraft pour nommer blocs, items, entités, textures…
//!
//! Les identifiants sont validés à la construction : on ne peut pas créer un
//! identifiant qui serait refusé par le registre Java côté Minecraft.

use std::fmt;
use std::str::FromStr;

/// Namespace implicite quand un identifiant est écrit sans préfixe.
pub const MINECRAFT_NAMESPACE: &str = "minecraft";

/// Namespace réservé à RustLib lui-même.
pub const RUSTLIB_NAMESPACE: &str = "rustlib";

/// Erreur de validation d'un [`ResourceLocation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// Le namespace ou le chemin est vide.
    Empty { part: &'static str },
    /// Caractère interdit (majuscule, espace, accent…).
    InvalidCharacter { part: &'static str, ch: char },
    /// Plus d'un `:` dans l'identifiant.
    TooManySeparators,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::Empty { part } => {
                write!(f, "identifiant invalide : la partie `{part}` est vide")
            }
            IdError::InvalidCharacter { part, ch } => write!(
                f,
                "identifiant invalide : caractère {ch:?} interdit dans la partie `{part}`"
            ),
            IdError::TooManySeparators => {
                write!(f, "identifiant invalide : plusieurs `:` (attendu namespace:path)")
            }
        }
    }
}

impl std::error::Error for IdError {}

/// Identifiant namespacé, par exemple `rustlib:copper_sword`.
///
/// ```
/// # use rustlib::id::ResourceLocation;
/// let id = ResourceLocation::parse("rustlib:copper_sword").unwrap();
/// assert_eq!(id.namespace(), "rustlib");
/// assert_eq!(id.path(), "copper_sword");
///
/// // Sans namespace, `minecraft` est implicite.
/// assert_eq!(ResourceLocation::parse("stone").unwrap().namespace(), "minecraft");
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceLocation {
    namespace: String,
    path: String,
}

impl ResourceLocation {
    /// Construit un identifiant à partir d'un namespace et d'un chemin.
    pub fn new(namespace: impl AsRef<str>, path: impl AsRef<str>) -> Result<Self, IdError> {
        let namespace = namespace.as_ref();
        let path = path.as_ref();
        validate_part("namespace", namespace)?;
        validate_part("path", path)?;
        Ok(Self {
            namespace: namespace.to_owned(),
            path: path.to_owned(),
        })
    }

    /// Construit un identifiant appartenant à votre mod : `monmod:nom`.
    ///
    /// Panique uniquement si `namespace` ou `path` est invalide, ce qui ne peut
    /// arriver que pour des littéraux fautifs (à utiliser dans les tests).
    pub fn mc(namespace: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        Self::new(namespace, path).unwrap_or_else(|e| panic!("identifiant invalide : {e}"))
    }

    /// Analyse `namespace:path` ou `path` (namespace `minecraft` implicite).
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        match raw.split_once(':') {
            Some((namespace, path)) => {
                if path.contains(':') {
                    return Err(IdError::TooManySeparators);
                }
                Self::new(namespace, path)
            }
            None => Self::new(MINECRAFT_NAMESPACE, raw),
        }
    }

    /// Namespace (`rustlib`, `minecraft`, …).
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Chemin dans le namespace (`copper_sword`).
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Le namespace implicite est-il utilisé ?
    pub fn is_minecraft(&self) -> bool {
        self.namespace == MINECRAFT_NAMESPACE
    }

    /// Change le namespace en gardant le chemin.
    pub fn with_namespace(&self, namespace: impl AsRef<str>) -> Result<Self, IdError> {
        Self::new(namespace, &self.path)
    }

    /// `true` si le chemin commence par `prefix` (utile pour les sous-dossiers).
    pub fn path_starts_with(&self, prefix: &str) -> bool {
        self.path.starts_with(prefix)
    }
}

impl fmt::Display for ResourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl fmt::Debug for ResourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceLocation({self})")
    }
}

impl FromStr for ResourceLocation {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Caractères acceptés par Minecraft dans un namespace ou un chemin.
fn validate_part(part: &'static str, value: &str) -> Result<(), IdError> {
    if value.is_empty() {
        return Err(IdError::Empty { part });
    }
    for ch in value.chars() {
        let ok = ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
            || matches!(ch, '_' | '-' | '.' | '/');
        if !ok {
            return Err(IdError::InvalidCharacter { part, ch });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_avec_namespace() {
        let id = ResourceLocation::parse("rustlib:copper_ingot").unwrap();
        assert_eq!(id.namespace(), "rustlib");
        assert_eq!(id.path(), "copper_ingot");
        assert_eq!(id.to_string(), "rustlib:copper_ingot");
    }

    #[test]
    fn parse_sans_namespace() {
        let id = ResourceLocation::parse("stone").unwrap();
        assert!(id.is_minecraft());
        assert_eq!(id.to_string(), "minecraft:stone");
    }

    #[test]
    fn sous_dossiers_autorises() {
        let id = ResourceLocation::parse("rustlib:block/copper_ore").unwrap();
        assert!(id.path_starts_with("block/"));
    }

    #[test]
    fn identifiants_invalides() {
        assert_eq!(
            ResourceLocation::parse("RustLib:Ore"),
            Err(IdError::InvalidCharacter {
                part: "namespace",
                ch: 'R'
            })
        );
        assert_eq!(
            ResourceLocation::parse("rustlib:copper ore"),
            Err(IdError::InvalidCharacter {
                part: "path",
                ch: ' '
            })
        );
        assert_eq!(
            ResourceLocation::parse("rustlib:a:b"),
            Err(IdError::TooManySeparators)
        );
        assert_eq!(
            ResourceLocation::parse("rustlib:"),
            Err(IdError::Empty { part: "path" })
        );
        assert_eq!(
            ResourceLocation::parse(""),
            Err(IdError::Empty { part: "path" })
        );
    }

    #[test]
    fn egalite_et_hachage() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ResourceLocation::parse("rustlib:ore").unwrap());
        assert!(set.contains(&ResourceLocation::parse("rustlib:ore").unwrap()));
        assert_eq!(
            ResourceLocation::parse("rustlib:ore").unwrap(),
            ResourceLocation::new("rustlib", "ore").unwrap()
        );
    }
}
