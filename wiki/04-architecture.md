# 4. Architecture

## Deux crates Rust, un jar

```text
┌──────────────────────────────────┐
│ rustlib.jar                      │  ce que la JVM charge
│   dev/rustlib/RustLib.class      │  façade Java (load/start/tick/close)
│   fabric.mod.json                │  bibliothèque Fabric sans entrypoint
│   native/rustlib.dll             │  = la cdylib d'un mod
└────────────────┬─────────────────┘
                 │ System.load + appels native
┌────────────────▼─────────────────┐
│ rustlib-mods (cdylib)              │  votre mod : logique + export_mod!
│   impl Mod, blocs, items         │
└────────────────┬─────────────────┘
                 │ appelle
┌────────────────▼─────────────────┐
│ rustlib-jni                      │  pont : handle, tick, JSON, événements
│   create / tick / destroy        │  sans logique métier
└────────────────┬─────────────────┘
                 │ dépend de
┌────────────────▼─────────────────┐
│ rustlib-core                     │  noyau : aucune dépendance
│   id  registry  event            │
│   tick  world  math              │
│   content  manifest  runtime     │
└──────────────────────────────────┘
```

Les noms de symboles JNI sont **fixes** (`Java_dev_rustlib_RustLib_*`) : le même
jar `rustlib.jar` fonctionne avec la cdylib de n'importe quel mod, générée par
`rustlib_jni::export_mod!`.

Le sens des dépendances est strictement descendant : le noyau **ne sait pas**
que la JVM existe. C'est ce qui permet de le tester entièrement, et de le
réutiliser (par exemple dans un outil hors ligne qui valide des datapacks).

## Carte des modules

| Module | Types principaux | Rôle |
| --- | --- | --- |
| `id` | `ResourceLocation`, `IdError` | Identifiants `namespace:path` validés |
| `manifest` | `ModMetadata`, `Version` | Identité du mod, namespaces réservés |
| `registry` | `Registry<T>`, `RegistryEntry` | Registre typé, gel après chargement |
| `event` | `EventBus`, `Event`, `EventPriority` | Bus typé, priorités, annulation |
| `events` | `BlockBreak`, `ServerTick`, … | Événements standard partagés |
| `tick` | `TickScheduler`, `TickReport` | Budget par tick, report du travail |
| `world` | `ChunkSection`, `SectionCache` | Données de blocs denses côté Rust |
| `math` | `Vec3`, `BlockPos`, `Aabb` | Maths sans allocation |
| `content` | `Block`, `Item`, `Tool` | Descriptions déclaratives du contenu |
| `runtime` | `Runtime` | Assemblage + export JSON |

## Cycle de vie

```text
Java                                 Rust
────────────────────────────────────────────────────────────
RustLib.start(budget)           →    (extraction + System.load)
  nativeCreate(budgetNanos)     →    (chargement de la dylib)
nativeCreate(budgetNanos)       →    Runtime::new + Runtime::load(mod)
                                     Runtime::finish_loading()  ← gel
nativeRegistries(handle)        ←    registries_json()
  (création des blocs/items)
chaque tick :
nativeTick(handle)              →    Runtime::tick()
                                       ├─ publie ServerTick
                                       └─ TickScheduler::run(budget)
PlayerBlockBreakEvents.BEFORE   →    nativeBlockBreak(...)
                                     = publie BlockBreak
                                     ← true/false (autorisé/annulé)
arrêt du serveur :
nativeDestroy(handle)           →    drop(Box<Runtime>)
```

## Pourquoi un handle `long` et pas une variable globale

Un serveur Minecraft peut charger puis décharger un monde (ou plusieurs
serveurs dans le même processus de test). Le handle rend l'état explicite :

* `nativeCreate` renvoie un `long` ; `0` signifie « échec du chargement » et la
  JVM a reçu une exception ;
* tous les autres appels reçoivent ce `long` et renvoient une valeur neutre
  (`-1`, `null`, `true`) si le handle est nul ;
* `nativeDestroy` libère l'état, une seule fois.

Aucune variable `static mut` n'est nécessaire, donc aucun verrou global dans le
tick.

## Le budget, pièce centrale

`Runtime::tick()` délègue à `TickScheduler::run(tick)`, qui :

1. trie les tâches échues par `(échéance, identifiant)` ;
2. exécute tant que le budget n'est pas consommé ;
3. **reporte** les tâches restantes au tick suivant ;
4. pour les tâches répétitives, saute les échéances manquées au lieu de les
   rattraper en rafale.

Le `TickReport` renvoyé contient `executed`, `deferred`, `overrun` et `elapsed`.
Un `deferred` durablement non nul signale un budget trop serré ; un `overrun`
signale une tâche trop longue pour le budget.

## État par tick : zéro allocation

Les décisions de conception qui rendent le tick prévisible :

* noms de tâches en `&'static str` ;
* `ChunkSection` : un `Box<[u16; 4096]>` pré-alloué, compteur de blocs non vides
  maintenu à l'écriture (donc O(1), pas de balayage) ;
* pas de `Vec` de travail recréé à chaque tick dans le noyau : l'ordonnanceur
  réutilise ses tampons (`std::mem::take` + `Vec::with_capacity`) ;
* la liste de tâches ne contient que des tâches actives (une tâche unique
  disparaît après exécution) ; le tri par tick porte donc sur une liste courte,
  et il est le seul coût O(n log n) de l'ordonnanceur.

→ [Registres](05-registres.md)
