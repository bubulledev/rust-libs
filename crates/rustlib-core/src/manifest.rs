//! Métadonnées d'un mod : identité, version, auteurs et dépendances.
//!
//! Ces métadonnées sont validées une seule fois, au chargement, pour que deux
//! mods ne puissent pas revendiquer le même identifiant ni squatter le
//! namespace `minecraft`.

use std::fmt;

use crate::id::{IdError, ResourceLocation, RUSTLIB_NAMESPACE, MINECRAFT_NAMESPACE};

/// Erreur de lecture d'une version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionError {
    /// La version ne contient pas exactement trois nombres (`1.2.3`).
    Malformed,
    /// Un des nombres n'est pas un entier non signé.
    NotANumber,
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionError::Malformed => {
                write!(f, "version invalide : format attendu `MAJEUR.MINEUR.CORRECTIF`")
            }
            VersionError::NotANumber => {
                write!(f, "version invalide : chaque partie doit être un entier")
            }
        }
    }
}

impl std::error::Error for VersionError {}

/// Version sémantique simplifiée (`MAJEUR.MINEUR.CORRECTIF`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(raw: &str) -> Result<Self, VersionError> {
        let mut parts = raw.split('.');
        let major = parts.next().ok_or(VersionError::Malformed)?;
        let minor = parts.next().ok_or(VersionError::Malformed)?;
        let patch = parts.next().ok_or(VersionError::Malformed)?;
        if parts.next().is_some() {
            return Err(VersionError::Malformed);
        }
        Ok(Self {
            major: major.parse().map_err(|_| VersionError::NotANumber)?,
            minor: minor.parse().map_err(|_| VersionError::NotANumber)?,
            patch: patch.parse().map_err(|_| VersionError::NotANumber)?,
        })
    }

    pub const fn major(self) -> u32 {
        self.major
    }

    pub const fn minor(self) -> u32 {
        self.minor
    }

    pub const fn patch(self) -> u32 {
        self.patch
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Erreur de construction des métadonnées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Le nom affiché est vide.
    EmptyName,
    /// Identifiant invalide.
    InvalidId(IdError),
    /// Namespace réservé à Minecraft ou à RustLib.
    ReservedNamespace(ResourceLocation),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::EmptyName => write!(f, "le nom du mod ne peut pas être vide"),
            ManifestError::InvalidId(err) => write!(f, "{err}"),
            ManifestError::ReservedNamespace(id) => write!(
                f,
                "le namespace de `{id}` est réservé : choisissez un identifiant propre au mod"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<IdError> for ManifestError {
    fn from(value: IdError) -> Self {
        ManifestError::InvalidId(value)
    }
}

/// Métadonnées d'un mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModMetadata {
    id: ResourceLocation,
    name: String,
    version: Version,
    authors: Vec<String>,
    description: String,
    license: Option<String>,
    minecraft_range: Option<String>,
}

impl ModMetadata {
    /// Commence la description d'un mod.
    ///
    /// ```
    /// # use rustlib::manifest::{ModMetadata, Version};
    /// # use rustlib::id::ResourceLocation;
    /// let meta = ModMetadata::builder(
    ///     ResourceLocation::mc("copper", "copper_expansion"),
    ///     "Copper Expansion",
    ///     Version::new(1, 0, 0),
    /// )
    /// .author("vous")
    /// .description("Ajoute du cuivre et des alliages.")
    /// .build()
    /// .unwrap();
    /// assert_eq!(meta.version().to_string(), "1.0.0");
    /// ```
    pub fn builder(
        id: ResourceLocation,
        name: impl Into<String>,
        version: Version,
    ) -> ModMetadataBuilder {
        ModMetadataBuilder {
            metadata: ModMetadata {
                id,
                name: name.into(),
                version,
                authors: Vec::new(),
                description: String::new(),
                license: None,
                minecraft_range: None,
            },
        }
    }

    /// Identifiant du mod (`copper:copper_expansion`).
    pub fn id(&self) -> &ResourceLocation {
        &self.id
    }

    /// Namespace déduit de l'identifiant.
    pub fn namespace(&self) -> &str {
        self.id.namespace()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn authors(&self) -> &[String] {
        &self.authors
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn license(&self) -> Option<&str> {
        self.license.as_deref()
    }

    /// Plage de versions de Minecraft supportée (par exemple `>=1.21 <1.22`).
    pub fn minecraft_range(&self) -> Option<&str> {
        self.minecraft_range.as_deref()
    }
}

/// Constructeur de [`ModMetadata`].
#[derive(Debug)]
pub struct ModMetadataBuilder {
    metadata: ModMetadata,
}

impl ModMetadataBuilder {
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.authors.push(author.into());
        self
    }

    pub fn authors(mut self, authors: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.metadata
            .authors
            .extend(authors.into_iter().map(Into::into));
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = description.into();
        self
    }

    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.metadata.license = Some(license.into());
        self
    }

    pub fn minecraft_versions(mut self, range: impl Into<String>) -> Self {
        self.metadata.minecraft_range = Some(range.into());
        self
    }

    pub fn build(self) -> Result<ModMetadata, ManifestError> {
        if self.metadata.name.trim().is_empty() {
            return Err(ManifestError::EmptyName);
        }
        let namespace = self.metadata.id.namespace();
        if namespace == MINECRAFT_NAMESPACE || namespace == RUSTLIB_NAMESPACE {
            return Err(ManifestError::ReservedNamespace(self.metadata.id));
        }
        Ok(self.metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> ResourceLocation {
        ResourceLocation::mc("copper", "copper_expansion")
    }

    #[test]
    fn version_parse_et_affiche() {
        assert_eq!(Version::parse("1.21.4").unwrap(), Version::new(1, 21, 4));
        assert_eq!(Version::parse("1.21.4").unwrap().to_string(), "1.21.4");
        assert_eq!(Version::parse("1.0"), Err(VersionError::Malformed));
        assert_eq!(Version::parse("1.0.0.0"), Err(VersionError::Malformed));
        assert_eq!(Version::parse("1.0.x"), Err(VersionError::NotANumber));
    }

    #[test]
    fn ordre_des_versions() {
        assert!(Version::new(1, 2, 0) < Version::new(1, 10, 0));
        assert!(Version::new(1, 10, 0) < Version::new(2, 0, 0));
    }

    #[test]
    fn metadonnees_completes() {
        let meta = ModMetadata::builder(id(), "Copper Expansion", Version::new(1, 0, 0))
            .author("vous")
            .authors(["ami"])
            .description("Cuivre et alliages.")
            .license("MIT")
            .minecraft_versions(">=1.21 <1.22")
            .build()
            .unwrap();
        assert_eq!(meta.namespace(), "copper");
        assert_eq!(meta.authors(), ["vous", "ami"]);
        assert_eq!(meta.license(), Some("MIT"));
        assert_eq!(meta.minecraft_range(), Some(">=1.21 <1.22"));
    }

    #[test]
    fn nom_vide_refuse() {
        assert_eq!(
            ModMetadata::builder(id(), "   ", Version::new(1, 0, 0)).build(),
            Err(ManifestError::EmptyName)
        );
    }

    #[test]
    fn namespaces_reserves() {
        assert_eq!(
            ModMetadata::builder(
                ResourceLocation::mc("minecraft", "truc"),
                "Truc",
                Version::new(1, 0, 0)
            )
            .build(),
            Err(ManifestError::ReservedNamespace(ResourceLocation::mc(
                "minecraft",
                "truc"
            )))
        );
        assert!(ModMetadata::builder(
            ResourceLocation::mc("rustlib", "truc"),
            "Truc",
            Version::new(1, 0, 0)
        )
        .build()
        .is_err());
    }
}
