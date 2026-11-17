# 3. Premier mod

Objectif : un mod qui ajoute un bloc, un item, réagit à un événement et exécute
une tâche périodique — puis qui se teste sans lancer Minecraft.

## 1. La structure minimale

```rust
use std::time::Duration;
use rustlib::prelude::*;

struct MonMod {
    meta: ModMetadata,
}

impl MonMod {
    fn new() -> Self {
        Self {
            meta: ModMetadata::builder(
                ResourceLocation::mc("monmod", "mon_mod"),
                "Mon Mod",
                Version::new(0, 1, 0),
            )
            .author("moi")
            .description("Mon premier mod RustLib.")
            .build()
            .expect("métadonnées valides"),
        }
    }
}
```

Deux règles de nommage, héritées de Minecraft :

* le **namespace** (`monmod`) est unique à votre mod et ne peut pas être
  `minecraft` ni `rustlib` (validé à la construction) ;
* le **chemin** n'accepte que des minuscules, chiffres, `_`, `-`, `.` et `/`.

`ResourceLocation::parse("monmod:mon_mod")` fait la même chose à partir d'une
chaîne, avec `minecraft` comme namespace implicite.

## 2. Enregistrer du contenu

```rust
impl Mod for MonMod {
    fn metadata(&self) -> ModMetadata {
        self.meta.clone()
    }

    fn register(&mut self, runtime: &mut Runtime) -> Result<(), RegistryError> {
        runtime.register_block(
            Block::builder(ResourceLocation::mc("monmod", "bloc_bizarre"))
                .material(Material::Stone)
                .hardness(2.0)
                .resistance(6.0)
                .light(7)
                .build()
                .expect("bloc valide"),
        )?;

        runtime.register_item(
            Item::builder(ResourceLocation::mc("monmod", "epee_bizarre"))
                .tool(Tool::new(ToolKind::Sword, ToolTier::Iron, 1.5, 3.0))
                .durability(250)
                .build()
                .expect("item valide"),
        )?;

        Ok(())
    }
}
```

`register` est appelée une seule fois, pendant le chargement. Un doublon
d'identifiant renvoie `RegistryError::Duplicate` : c'est volontaire, deux mods ne
peuvent pas revendiquer le même identifiant silencieusement.

Notez les `.expect(...)` : les constructeurs `Block`/`Item` valident leurs
paramètres *au moment du `build()`* (`ContentError`), donc une description
correcte échoue dès le premier test, pas au chargement en production. Seul
l'enregistrement lui-même peut renvoyer `RegistryError`.

## 3. Réagir à un événement

```rust
use rustlib::events::BlockBreak;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

let compteur = Arc::new(AtomicU64::new(0));
let c = compteur.clone();

runtime.events_mut().subscribe::<BlockBreak>(EventPriority::Normal, move |event| {
    // Un gestionnaire reçoit `&mut E` : il peut lire et annuler.
    if event.block == ResourceLocation::mc("monmod", "bloc_bizarre") {
        c.fetch_add(1, Ordering::Relaxed);
        event.cancel(); // la casse sera refusée par la JVM
    }
});
```

Points importants :

* le bus est **typé** : `subscribe::<BlockBreak>` ne reçoit que des `BlockBreak` ;
* `Highest` s'exécute avant `Lowest`, et à priorité égale l'ordre d'abonnement
  est conservé ;
* après une annulation, les gestionnaires suivants sont ignorés — sauf ceux
  abonnés avec `subscribe_including_cancelled`, prévus pour l'observation.

## 4. Planifier une tâche bornée

```rust
runtime.scheduler_mut().schedule_repeating(20, "mon_balayage", move |ctx| {
    // ctx.tick       : numéro du tick
    // ctx.task       : "mon_balayage"
    // ctx.budget_remaining : temps restant estimé pour ce tick
    println!("tick {} (budget restant {:?})", ctx.tick, ctx.budget_remaining);
});
```

Le nom est un `&'static str` : aucune allocation au moment du tick. Une tâche
unique se planifie avec `schedule_after(delai, nom, closure)` et s'annule avec
`cancel(id)`.

## 5. Tester sans Minecraft

```rust
#[test]
fn le_mod_se_charge() {
    let mut runtime = Runtime::new(Duration::from_millis(2));
    runtime.load(&mut MonMod::new()).unwrap();
    runtime.finish_loading();

    assert_eq!(runtime.blocks().len(), 1);
    assert_eq!(runtime.items().len(), 1);

    let mut event = BlockBreak::new(
        ResourceLocation::mc("monmod", "bloc_bizarre"),
        BlockPos::new(0, 64, 0),
        "joueur",
    );
    assert!(!runtime.post(&mut event), "la casse doit être annulée");
}
```

C'est le principal intérêt de la séparation : **toute la logique du mod est
testable sans serveur ni JVM**. Les fonctions JNI, décrites en
[page 8](08-pont-jni-fabric.md), ne doivent contenir aucune règle métier.

## 6. Brancher la JVM

Une seule ligne expose le mod à Java : la macro génère les cinq symboles JNI
attendus par le jar `rustlib.jar`.

```rust
rustlib_jni::export_mod!(|| Box::new(MonMod::new()));
```

Puis, côté Java :

```java
RustLib lib = RustLib.start(2_000_000L);   // 2 ms par tick
String json = lib.registriesJson();        // blocs et items à créer
lib.tick();                               // une fois par tick
```

`./build-jar.sh` construit le jar et vérifie la chaîne complète. Détails en
[page 8](08-pont-jni-fabric.md).

## Erreurs fréquentes

| Symptôme | Cause |
| --- | --- |
| `RegistryError::Frozen` | `register_block` appelé après `finish_loading()` |
| `LoadError::DuplicateMod` | deux mods déclarent le même identifiant |
| `ManifestError::ReservedNamespace` | namespace `minecraft` ou `rustlib` utilisé |
| `ContentError::InvalidLightLevel` | `light(16)` ou plus |
| `ContentError::InvalidStackSize` | `stack_size(0)` ou `stack_size(65)` |

→ [Architecture](04-architecture.md)
