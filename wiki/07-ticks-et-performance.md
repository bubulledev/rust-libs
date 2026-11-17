# 7. Ticks et performance

## Le seul chiffre qui compte

Un serveur Minecraft tourne à 20 ticks par seconde : **50 ms par tick**. Tout ce
que fait votre mod doit tenir dans ce budget, sinon le serveur ralentit et les
joueurs voient des entités qui sautent.

Conséquence directe : « faire plus de choses » n'est jamais une optimisation.
« Faire tenir plus de choses dans 50 ms » en est une.

## Le coût de la frontière JNI

Chaque appel JNI implique :

* une transition JVM → natif (barrière mémoire, sauvegarde de contexte) ;
* une éventuelle conversion de types (chaînes en UTF-8 modifié, tableaux) ;
* une vérification d'exception au retour.

Pour un aller-retour trivial, ce coût dépasse largement celui d'un appel de
méthode Java. Un mod qui lit un bloc en traversant la frontière **à chaque
accès** sera plus lent que sa version Java, quel que soit le compilateur.

D'où la règle de RustLib :

> **Un appel JNI par tick, et des données copiées une fois.**

## Le budget de tick

```rust
use std::time::Duration;

let mut runtime = Runtime::new(Duration::from_millis(2));
```

Deux millisecondes sur cinquante : 4 % du tick pour vos tâches Rust. À chaque
tick, l'ordonnanceur exécute les tâches échues jusqu'à épuisement du budget,
puis reporte le reste :

```rust
let report = runtime.tick();
if report.deferred > 0 && report.executed > 0 {
    // Le budget est durablement trop serré : réduisez la fréquence,
    // découpez le travail, ou augmentez légèrement le budget.
}
if report.overrun {
    // Une tâche a dépassé le budget à elle seule : elle doit être découpée.
}
```

### Le report, pas la dette

Une tâche répétitive dont l'échéance est manquée est **recalée** sur la grille
initiale (`échéance + n × période`), jamais rejouée en rafale. Un serveur qui
gèle 2 secondes ne déclenche donc pas 40 exécutions d'un coup au réveil.

### Découper plutôt qu'augmenter

```rust
// Mauvais : 40 ms de calcul dans une seule tâche
scheduler.schedule_repeating(20, "scan_complet", move |_| scanner_tout());

// Mieux : un quart du travail par tick
scheduler.schedule_repeating(5, "scan_quart", move |_| scanner_quart());
```

## Là où le Rust gagne réellement

| Cas | Gain attendu | Pourquoi |
| --- | --- | --- |
| Boucles sur des tableaux de blocs | élevé | accès contigus, pas de boxing, pas d'itérateurs virtuels |
| Recherche de chemin, simulation de fluides | élevé | code natif optimisé, structures compactes |
| Sérialisation de données volumineuses | moyen à élevé | une seule traversée, pas d'objets intermédiaires |
| Traitement de millions de positions | élevé | SIMD via autovectorisation, pas d'allocation par élément |
| Accès ponctuels à l'API Minecraft | **négatif** | le coût JNI dépasse le gain |
| Rendu, entités, réseau | aucun | hors périmètre |

Autrement dit : RustLib ne rend pas Minecraft plus rapide. Il rend **le calcul de
votre mod** plus rapide, à condition que ce calcul soit volumineux, régulier et
qu'il travaille sur des données déjà copiées côté Rust.

## Le cache de sections

`SectionCache` garde les sections de chunk reçues de la JVM :

```rust
use rustlib::world::{ChunkSection, SectionCache, SectionPos, WorldView};
use rustlib::math::BlockPos;

let mut cache = SectionCache::new();
let mut section = ChunkSection::new();
section.set(3, 4, 5, 42);
cache.insert(SectionPos::new(0, 0, 0), section);

// Le même accès que côté Java, mais en Rust et sur un tableau contigu.
assert_eq!(cache.block_state_at(BlockPos::new(3, 4, 5)), Some(42));
assert_eq!(cache.block_state_at(BlockPos::new(100, 4, 5)), None); // chunk absent
```

Coût mémoire : 8 Kio par section pleine (`4096 × u16`). Le compteur de blocs non
vides est maintenu à l'écriture, donc `non_air_count()` est O(1).

## Mesurer avant de conclure

1. Compilez en `--release` (les chiffres du profil `debug` n'ont aucun sens).
2. Mesurez avec `TickReport::elapsed` et `is_healthy()` dans votre propre
   serveur, sur des scénarios réalistes (50 joueurs, machines actives).
3. Comparez **toujours** à la même logique en Java, dans les mêmes conditions.
   Sinon vous mesurez la machine, pas le langage.
4. Un gain inférieur à ~10 % ne justifie pas la complexité d'un pont natif.

Aucun chiffre absolu n'est publié ici : les débits dépendent du CPU, de la
charge du serveur et de la taille des données. La seule mesure valable est celle
de votre scénario.

→ [Pont JNI / Fabric](08-pont-jni-fabric.md)
