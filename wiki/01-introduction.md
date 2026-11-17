# 1. Introduction

## Le problème

Un mod Minecraft se heurte à deux murs :

1. **La JVM impose un plafond.** Les tâches gourmandes d'un mod (recherche de
   chemin, simulation de machines, traitement de chunks entiers) consomment le
   même fil que le tick du serveur. Dépasser 50 ms par tick et le serveur
   « lag ».
2. **Le code critique est dispersé.** Chaque mod réinvente son ordonnanceur,
   son cache de blocs, sa gestion de priorités d'événements, souvent avec des
   allocations et des accès non bornés.

## Ce que RustLib apporte

RustLib est une bibliothèque Rust avec quatre briques :

| Brique | Module | Ce qu'elle règle |
| --- | --- | --- |
| Registres | `rustlib::registry` | Un identifiant ne peut pas être enregistré deux fois ; les registres sont gelés après le chargement. |
| Événements | `rustlib::event` | Ordre d'exécution déterministe (priorité puis ordre d'abonnement), annulation propre. |
| Ordonnanceur | `rustlib::tick` | Un budget de temps par tick : le travail restant est reporté au lieu de faire exploser le tick. |
| Monde | `rustlib::world` | Sections de chunk denses de 4096 `u16` (8 Kio), compteur de blocs vides en O(1). |

Le tout **sans aucune dépendance externe** dans le noyau : pas de `serde`, pas
de runtime asynchrone, pas de bibliothèque mathématique. Vous gardez la maîtrise
complète de ce qui est compilé et de ce qui tourne dans le tick.

## La frontière, très concrète

Traverser la frontière JNI coûte cher : conversion de types, vérifications,
barrières mémoire. Un appel JNI par lecture de bloc rendrait votre mod **plus
lent** que sa version Java.

Le pont de RustLib est donc dessiné pour un seul appel par tick :

```text
tick Java ──▶ nativeTick(handle) ──▶ Runtime::tick() ──▶ tâches Rust
```

Le reste (les données) est copié **une fois** : les registres partent en JSON au
chargement, les sections de chunk utiles sont copiées quand elles se chargent.
Pendant le tick, tout se passe côté Rust, sur des tableaux contigus.

## Ce que RustLib ne fait pas

* **Pas de rendu.** Les shaders, les modèles, les particules restent en Java.
* **Pas de remplacement de mixins.** Rust ne peut pas réécrire les classes
  Java : vos hooks passent par les API Fabric, puis par le pont.
* **Pas de magie de performance.** Déplacer du code en Rust n'accélère que ce
  qui était réellement coûteux et mal optimisé. La page
  [Ticks et performance](07-ticks-et-performance.md) explique comment mesurer
  avant de se réjouir.

## Suite

→ [Installation](02-installation.md)
