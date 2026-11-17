# 2. Installation

## Prérequis

| Outil | Version | Pour quoi |
| --- | --- | --- |
| Rust (stable) | 1.75 ou plus | Bibliothèque et mod |
| Cargo | fourni avec Rust | Compilation, tests |
| JDK | 21 (ou celui de votre version de Minecraft) | Mod Fabric côté Java |
| Gradle | via le modèle Fabric (Loom) | Construction du jar |
| Fabric API | version de votre Minecraft | Hooks Java (`ServerTickEvents`, `PlayerBlockBreakEvents`) |

RustLib n'exige **aucune** dépendance Rust externe : le noyau compile avec la
bibliothèque standard seule.

## Compiler le dépôt

```bash
cargo test --workspace     # 58 tests (54 unitaires + 4 doctests)
./build-jar.sh             # produit dist/rustlib-0.1.0.jar et le vérifie
```

`build-jar.sh` enchaîne : compilation Rust en `release`, compilation des classes
Java, assemblage du jar, puis **vérification de bout en bout** (le jar charge son
natif, crée le mod, envoie des ticks et teste une casse de bloc annulée).

| Artefact | Contenu |
| --- | --- |
| `dist/rustlib-0.1.0.jar` | `dev/rustlib/RustLib.class`, `fabric.mod.json`, `native/rustlib.dll` |
| `target/release/rustlib_mods.dll` | la bibliothèque native seule (Windows ; `.so` / `.dylib` ailleurs) |

Le nom de la dylib vient de `[lib] name = "rustlib_mods"` ; le script la renomme
en `rustlib.dll` (ou `librustlib.so`, `librustlib.dylib`) à l'intérieur du jar,
car c'est le nom que la classe Java charge.

La version du jar n'existe qu'à un seul endroit : `version` dans
`java/src/main/resources/fabric.mod.json`. Le script en déduit le nom de
l'artefact et le manifeste, donc rien à synchroniser.

## Configurer votre crate de mod

```toml
[package]
name = "mon-mod"
version = "0.1.0"
edition = "2021"

[lib]
name = "mon_mod"
# cdylib : la bibliothèque dynamique pour la JVM
# rlib   : permet `cargo test` sur le code du mod
crate-type = ["cdylib", "rlib"]

[dependencies]
rustlib-core = { path = "../rustlib/crates/rustlib-core" }
rustlib-jni = { path = "../rustlib/crates/rustlib-jni" }
```

`jni` n'est pas nécessaire dans votre crate : la macro `export_mod!` du pont
y fait référence via `$crate::jni`.

En production, publiez `rustlib-core` et `rustlib-jni` sur votre registre privé ou
utilisez `git = "…"` ; les chemins relatifs ci-dessus sont ceux du dépôt.

## Réglages de compilation recommandés

```toml
[profile.release]
lto = "thin"          # optimisations inter-crates (déjà actif dans ce dépôt)
codegen-units = 1     # meilleur code généré, compilation plus lente
panic = "abort"       # voir ci-dessous
```

`panic = "abort"` n'est pas décoratif : un `panic` qui se propage à travers une
frontière `extern "system"` (donc JNI) est un comportement indéfini. Un mod Rust
doit échouer proprement — `abort` est plus sûr qu'un déroulement de pile dans la
JVM. Gérez vos erreurs avec `Result` et renvoyez un handle nul plutôt que de
paniquer.

## Charger la bibliothèque depuis Java

Le jar s'en occupe : `RustLib.start()` extrait `native/rustlib.dll` du jar vers un
dossier temporaire, puis appelle `System.load`. Détails et alternatives dans
[Pont JNI / Fabric](08-pont-jni-fabric.md).

La bibliothèque doit être compilée pour la plateforme du serveur : un `.dll`
Windows ne se charge pas sur un serveur Linux. Prévoyez un jar par plateforme si
votre mod est distribué publiquement.

## Vérifier que tout est en place

```bash
cargo test -p rustlib-core     # noyau : registres, événements, ticks, chunks
cargo test -p rustlib-mods      # logique du mod d'exemple, sans serveur
```

Les tests du mod d'exemple sont la partie la plus utile à imiter : ils
construisent un `Runtime`, chargent le mod, puis simulent des événements et des
ticks sans Minecraft.

→ [Premier mod](03-premier-mod.md)
