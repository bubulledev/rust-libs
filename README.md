# RustLib

Bibliothèque Rust pour écrire la logique de mods Minecraft Fabric, avec un pont
JNI minimal, un exemple de mod complet et un wiki en français.

RustLib ne remplace pas Minecraft : votre mod reste un mod Fabric classique côté
Java (rendu, réseau, protocole, logique vanilla). RustLib fournit la couche où
vivent les décisions : registres de blocs et d'items, bus d'événements avec
priorités et annulation, ordonnanceur borné par un budget de tick, sections de
chunk denses pour les calculs spatiaux.

```text
        votre mod Fabric (Java)                    RustLib (Rust)
┌──────────────────────────────────┐        ┌──────────────────────────────┐
│ entrypoint, mixins, rendu        │        │ registres  (blocs, items)    │
│ un appel JNI par tick ───────────┼───────▶│ événements (priorités,       │
│                                  │  JNI   │             annulation)      │
│                                  │◀───────┤ ticks      (budget borné)    │
└──────────────────────────────────┘  JSON  │ monde      (sections, cache) │
                                            └──────────────────────────────┘
```

## Sommaire

- [Ce que RustLib fait, et ce qu'il ne fait pas](#ce-que-rustlib-fait-et-ce-quil-ne-fait-pas)
- [Comment ça marche](#comment-ça-marche)
- [Contenu du dépôt](#contenu-du-dépôt)
- [Démarrage rapide](#démarrage-rapide)
- [Écrire un mod en Rust](#écrire-un-mod-en-rust)
- [Le mod Fabric : RustLib Mods](#le-mod-fabric--rustlib-mods)
- [API en un coup d'œil](#api-en-un-coup-dœil)
- [Tests](#tests)
- [Wiki](#wiki)
- [Publier](#publier)
- [Limites assumées](#limites-assumées)
- [Licence](#licence)

## Ce que RustLib fait, et ce qu'il ne fait pas

**Ce qu'elle fait**

- **Registres** : blocs (matériau, dureté, résistance, lumière, drops) et items
  (taille de pile, outil, durabilité), avec gel après chargement et export JSON
  pour le côté Java.
- **Événements** : bus typé (`BlockBreak`, `BlockPlace`, `PlayerDamage`,
  `ServerTick`, `PlayerJoin`, `PlayerQuit`), priorités, annulation coopérative.
- **Ticks** : ordonnanceur de tâches ponctuelles ou répétées, borné par un budget
  en nanosecondes, avec un `TickReport` (tâches exécutées, temps consommé,
  travail restant) pour mesurer au lieu de supposer.
- **Monde** : sections de chunk denses (`ChunkSection`, `SectionCache`) et
  mathématiques utilitaires pour les calculs spatiaux hors de la JVM.
- **Pont JNI** : cinq symboles à noms fixes, une macro (`export_mod!`), un seul
  natif par JVM.

**Ce qu'elle ne fait pas**

- Pas de rendu, pas de client, pas de réseau : RustLib vise la logique serveur.
- Pas de remplacement de la Fabric API : elle s'ajoute à un mod Fabric normal.
- Pas de magie de performance : Rust n'est rapide que si le travail déplacé est
  réellement coûteux, et chaque appel JNI a un coût fixe.

## Comment ça marche

### Les crates

| Crate | Rôle |
| --- | --- |
| `rustlib-core` (lib `rustlib`) | Le noyau, **sans aucune dépendance** : registres, événements, ticks, monde, maths. |
| `rustlib-jni` (lib `rustlib_jni`) | Le pont : dépend de `jni` et de `rustlib-core`, exporte la macro `export_mod!`. |
| `rustlib-mods` (`examples/rustlib-mods`) | Le mod de référence, `cdylib`, consommateur des deux crates. |

### Le flux d'un tick

1. Fabric appelle `ServerTickEvents.END_SERVER_TICK` une fois par tick de serveur.
2. Le Java appelle `lib.tick()` ; `RustLib` entre en JNI.
3. `Runtime::tick()` exécute les tâches dues dans la limite du budget
   (par défaut 2 ms) et renvoie un `TickReport` sérialisé en un `long`.
4. Les tâches qui n'ont pas tenu dans le budget sont reportées au tick suivant :
   le serveur ne prend jamais un retard illimité à cause d'un mod.

### Le pont JNI

La macro `rustlib_jni::export_mod!` génère exactement cinq symboles, dont les
noms sont fixés par la classe Java `dev.rustlib.RustLib` :

| Symbole JNI | Rôle |
| --- | --- |
| `Java_dev_rustlib_RustLib_nativeCreate` | construit le `Runtime`, construit et charge le mod |
| `Java_dev_rustlib_RustLib_nativeTick` | exécute un tick, renvoie le rapport |
| `Java_dev_rustlib_RustLib_nativeRegistriesJson` | exporte les registres en JSON |
| `Java_dev_rustlib_RustLib_nativeBlockBreak` | poste un `BlockBreak`, renvoie « autorisé ? » |
| `Java_dev_rustlib_RustLib_nativeDestroy` | détruit le runtime |

Point important : **un seul natif par JVM**. Charger un second natif différent
lève une `IllegalStateException`, plutôt que de laisser deux runtimes se
marcher dessus silencieusement.

## Contenu du dépôt

| Chemin | Rôle |
| --- | --- |
| `crates/rustlib-core` | Le noyau : registres, événements, ticks, monde, maths. **Aucune dépendance.** |
| `crates/rustlib-jni` | Le pont JNI : `create`, `tick`, `registries_json`, `on_block_break`, `destroy`. |
| `examples/rustlib-mods` | Mod **RustLib Mods** : dépend des crates `rustlib` et `rustlib-jni`, `export_mod!` + tests. |
| `java/` | La classe `dev.rustlib.RustLib` et le `fabric.mod.json` du jar de la bibliothèque. |
| `build-jar.sh` | Construit `dist/rustlib-0.1.0.jar`, le vérifie de bout en bout et publie la coordonnée Maven locale. |
| `fabric-mod/` | Le mod Fabric complet : Gradle fabric-loom, entrypoint, jar avec natif + bibliothèque imbriquée. |
| `wiki/` | Le wiki : 10 pages, du premier mod aux limites de l'approche. |
| `tools/` | Générateur du site du wiki (`npm run build:wiki`). |
| `modrinth.md` / `modrinth.fr.md` | Les fiches de publication (anglais / français). |

## Démarrage rapide

Prérequis : Rust ≥ 1.75, un JDK 21, et npm uniquement pour le site du wiki.

```bash
cargo test --workspace              # 58 tests, sans serveur Minecraft
CARGO_FLAGS=--offline ./build-jar.sh # dist/rustlib-0.1.0.jar + vérification jar → JNI → Rust
```

`build-jar.sh` fait six choses, dans l'ordre :

1. compile la cdylib du mod (`cargo build --release -p rustlib-mods`) ;
2. localise `javac`, `java` et `jar` (JAVA_HOME, PATH, installations courantes) ;
3. compile les classes Java de la bibliothèque ;
4. assemble le jar : classes, `fabric.mod.json`, natif sous `native/`, licences
   MIT et Apache-2.0 sous `META-INF/` ;
5. exécute un test de fumée réel (`RustLibSmokeTest`) : chargement du jar,
   chargement du natif, ticks, quota d'événements ;
6. publie `dev.rustlib:rustlib:<version>` (jar + POM) dans `dist/maven`, le
   dépôt local que Gradle consomme pour le mod.

## Écrire un mod en Rust

Le mod est un type qui implémente `Mod` : il décrit ses métadonnées, enregistre
son contenu et branche ses gestionnaires.

```rust
use rustlib::prelude::*;
use std::time::Duration;

struct MonMod { meta: ModMetadata }

impl MonMod {
    fn new() -> Self {
        Self {
            meta: ModMetadata::builder(
                ResourceLocation::mc("monmod", "mon_mod"),
                "Mon Mod",
                Version::new(0, 1, 0),
            )
            .description("Exemple minimal.")
            .build()
            .unwrap(),
        }
    }
}

impl Mod for MonMod {
    fn metadata(&self) -> ModMetadata { self.meta.clone() }

    fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError> {
        runtime.register_block(
            Block::builder(ResourceLocation::mc("monmod", "bloc_bizarre"))
                .material(Material::Stone)
                .hardness(2.0)
                .build()
                .unwrap(),
        )?;
        Ok(())
    }
}

let mut runtime = Runtime::new(Duration::from_millis(2));
runtime.load(&mut MonMod::new()).unwrap();
runtime.finish_loading();
runtime.tick();
```

Règles à garder en tête :

- le namespace `minecraft` et le namespace `rustlib` sont réservés ; utilisez le
  vôtre (`monmod` ci-dessus) ;
- la logique métier vit dans du Rust pur, testable sans JVM ; le pont JNI reste
  mince, c'est le seul endroit non testable hors serveur ;
- un seul `RustLib` par JVM.

## Le mod Fabric : RustLib Mods

`fabric-mod/` transforme l'exemple en vrai mod Fabric, chargeable dans `mods/`.

```bash
./build-jar.sh             # d'abord la bibliothèque : elle publie la coordonnée Maven
cd fabric-mod
./gradlew build            # build/libs/rustlib-mods-0.1.0.jar
./gradlew runServer        # serveur de dev (accepter la EULA dans run/eula.txt)
```

Ce que fait le mod :

- `fabric.mod.json` : id `rustlib_mods`, entrypoint
  `dev.rustlib.fabric.RustLibMods`, dépendances `fabricloader`, `fabric-api`,
  `minecraft ~1.21`, `java >=21`, `rustlib >=0.1.0` ;
- `RustLib.start()` au démarrage, `lib.tick()` sur chaque
  `ServerTickEvents.END_SERVER_TICK`, `lib.close()` sur `SERVER_STOPPED` ;
- un natif absent (autre plateforme) désactive le mod proprement au lieu de
  faire tomber le serveur.

RustLib est une **vraie dépendance**, pas une copie de fichier :

```gradle
repositories { maven { url = uri('../dist/maven') } }
dependencies { modImplementation include("dev.rustlib:rustlib:0.1.0") }
```

Côté Rust, le mod ne fait que consommer la bibliothèque, avec une contrainte de
version :

```toml
[dependencies]
rustlib = { package = "rustlib-core", path = "../../crates/rustlib-core", version = "0.1.0" }
rustlib-jni = { path = "../../crates/rustlib-jni", version = "0.1.0" }
```

Le build Gradle lance `cargo build --release -p rustlib-mods` puis copie le
natif dans le jar. Après une modification de RustLib à version inchangée,
ajoutez `--refresh-dependencies` (Gradle garde en cache les artefacts de version
fixe).

### Ce que contient le jar du mod

```text
fabric.mod.json                     id, entrypoint, depends, jars
dev/rustlib/fabric/RustLibMods.class  entrypoint : start + tick + close
native/rustlib_mods.dll             logique Rust du mod (Windows x64)
META-INF/jars/rustlib-0.1.0.jar     la bibliothèque, ajoutée par Loom (jar-in-jar)
```

Le jar est autonome : l'utilisateur n'installe que Fabric API en plus.

## API en un coup d'œil

| Besoin | Rust | Java |
| --- | --- | --- |
| Démarrer | `Runtime::new(Duration)`, `load`, `finish_loading` | `RustLib.start()` |
| Ticker | `Runtime::tick() -> TickReport` | `lib.tick() -> long` |
| Enregistrer | `register_block`, `register_item` | — (côté Rust) |
| Exporter | `registries_json()` | `lib.registriesJson()` |
| Événements | `events_mut().subscribe::<E>(priorité, closure)` | `lib.blockBreak(x, y, z, bloc, joueur)` |
| Planifier | `scheduler_mut().schedule_repeating(ticks, nom, closure)` | — (côté Rust) |
| Arrêter | `drop(runtime)` | `lib.close()` |

La référence exhaustive (toutes les signatures) est dans
[`wiki/09-reference-api.md`](wiki/09-reference-api.md).

## Tests

```bash
cargo test --workspace        # 54 tests unitaires + 4 doctests
```

Les tests couvrent les registres, les priorités et l'annulation d'événements,
les fenêtres de ticks, les sections de chunk, les métadonnées et le mod de
référence (quota de minage inclus). Aucun test ne demande de serveur Minecraft :
c'est justement l'intérêt d'avoir séparé la logique de la JVM.

Le test de fumée du jar (`java/src/test/java/dev/rustlib/RustLibSmokeTest.java`)
est lancé par `build-jar.sh` et vérifie la chaîne réelle : jar → natif → runtime
Rust → tick → événement annulé.

## Wiki

| Page | Sujet |
| --- | --- |
| [Introduction](wiki/01-introduction.md) | Ce que RustLib fait, et ce qu'il ne fait pas |
| [Installation](wiki/02-installation.md) | Outils, compilation, artefacts |
| [Premier mod](wiki/03-premier-mod.md) | Bloc, item, événement, tâche, en 30 minutes |
| [Architecture](wiki/04-architecture.md) | Les crates et le flux d'un tick |
| [Registres](wiki/05-registres.md) | Blocs, items, gel et export JSON |
| [Événements](wiki/06-evenements.md) | Priorités, annulation, événements standard |
| [Ticks et performance](wiki/07-ticks-et-performance.md) | Budget de tick et gain réel |
| [Pont JNI / Fabric](wiki/08-pont-jni-fabric.md) | Convention de symboles et code Java |
| [Référence API](wiki/09-reference-api.md) | Toutes les signatures publiques |
| [Limites et FAQ](wiki/10-limites-faq.md) | Ce que le Rust ne peut pas faire ici |

### Site du wiki

Les mêmes pages rendues en site statique :

```bash
npm install
npm run build:wiki         # dist/wiki/ (HTML + CSS + manifest de déploiement)
```

## Publier

- **Modrinth** : le fichier à déposer est `dist/rustlib-mods-0.1.0.jar`
  (type Mod, chargeur Fabric, Minecraft 1.21.1). Les textes de fiche sont dans
  [`modrinth.md`](modrinth.md) (anglais) et [`modrinth.fr.md`](modrinth.fr.md)
  (français), avec les mêmes limites annoncées.
- **Bibliothèque seule** : `dist/rustlib-0.1.0.jar` est destiné au jar-in-jar
  des mods, ou à un usage hors Minecraft ; publié séparément, ce serait un
  projet « Library » dont RustLib Mods dépendrait.

## Limites assumées

- Pas de rendu, pas de client : RustLib vise la logique serveur.
- **Le mod ne crée pas encore ses blocs et items dans les registres Minecraft** :
  ils existent dans le runtime Rust et dans le JSON exporté, mais pas en jeu. Le
  câblage registre par registre reste à faire.
- **Le natif livré est Windows x64 uniquement.** Sous Linux ou macOS, le mod se
  charge, journalise « RustLib indisponible » et reste inactif.
- Chaque appel JNI coûte plus cher qu'un appel Java : un appel par tick, pas un
  par bloc.
- Rust n'est pas « plus rapide » gratuitement : le gain vient des tâches
  gourmandes déplacées hors de la JVM, mesurées avec `TickReport`.

## Licence

Double licence, au choix : **MIT** ([LICENSE-MIT](LICENSE-MIT)) ou
**Apache-2.0** ([LICENSE-APACHE](LICENSE-APACHE)).
