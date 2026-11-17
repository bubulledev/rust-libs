# Fiche Modrinth — RustLib

Tout ce qui suit est prêt à coller dans l'assistant de publication Modrinth.
Le résumé et la description sont en français ; dis-moi si tu veux la version
anglaise (recommandée pour la portée internationale).

## Métadonnées à remplir

| Champ | Valeur |
| --- | --- |
| Slug | `rustlib` |
| Nom du projet | RustLib |
| Type de projet | Mod |
| Résumé | voir ci-dessous (240 caractères) |
| Loaders | Fabric |
| Versions de jeu | 1.21.x (le pont ne dépend d'aucune classe Minecraft : la compatibilité vient de votre mod) |
| Catégories | Library, Utility |
| Licence | `MIT OR Apache-2.0` → choisir « Custom license » avec ce texte, ou « MIT » si un seul suffit |
| Liens | Site/wiki : votre wiki ; Source : votre dépôt Git |
| Fichier de version | `dist/rustlib-0.1.0.jar` (un jar par plateforme : Windows, Linux, macOS) |

## Résumé (champ « Summary »)

> Bibliothèque pour écrire la logique de vos mods Fabric en Rust : registres,
> événements à priorité, ticks bornés par un budget de temps, accès aux chunks.
> Livrée en jar, avec sa bibliothèque native embarquée.

## Description (corps de la page)

# RustLib — la logique de vos mods Fabric, en Rust

**RustLib n'est pas un mod de gameplay : c'est une bibliothèque pour
développeurs de mods.** Elle fournit la couche Rust d'un mod Fabric : registres
de blocs et d'items, bus d'événements, ordonnanceur borné par tick et cache de
sections de chunk. Le rendu, le réseau et la logique vanilla restent en Java.

Si vous jouez et cherchez un mod à installer : **passez votre chemin**, RustLib
ne s'installe pas seul et n'ajoute aucun contenu.

## Le problème

Un mod Minecraft tourne sur le fil du serveur, avec 50 ms par tick pour tout le
monde. Dès qu'un mod fait du calcul lourd — simulation de machines, recherche de
chemin, traitement de chunks entiers — il mange ce budget et le serveur « lag ».
Souvent, ce code réinvente aussi son ordonnanceur, son cache de blocs et sa
gestion de priorités, avec des allocations plein le tick.

## Ce que RustLib apporte

| Brique | Ce qu'elle règle |
| --- | --- |
| **Registres** | Un identifiant ne peut pas être enregistré deux fois ; les registres sont gelés après le chargement, pour ne jamais produire un monde illisible. |
| **Événements** | Cinq niveaux de priorité, ordre d'exécution déterministe, annulation (casser un bloc, subir des dégâts). Événements standard fournis : `BlockBreak`, `BlockPlace`, `PlayerDamage`, `ServerTick`, `PlayerJoin`, `PlayerQuit`. |
| **Ticks bornés** | Un budget de temps par tick : le travail restant est **reporté** au tick suivant au lieu de faire exploser le tick. Les échéances manquées ne sont jamais rattrapées en rafale. |
| **Monde** | Sections de chunk denses (4096 états en `u16`, 8 Kio), compteur de blocs non vides en O(1), cache alimenté une fois par la JVM. |
| **Contenu déclaratif** | Blocs et items décrits en Rust (matériau, dureté, résistance, lumière, butin, outils, durabilité, nourriture), puis exportés en JSON au chargement vers le registre Java. |

Le noyau (`rustlib-core`) **n'a aucune dépendance externe** : ni `serde`, ni
runtime asynchrone, ni bibliothèque mathématique. Vous gardez la maîtrise de ce
qui est compilé et de ce qui tourne dans le tick.

## Comment ça marche

```text
rustlib.jar (JVM) ── un appel JNI par tick ──▶ runtime Rust
   RustLib.class                              événements
   natif embarqué                             calculs spatiaux
   fabric.mod.json                            simulations bornées
```

La frontière JNI est chère : un aller-retour par accès bloc rendrait votre mod
plus lent que sa version Java. RustLib est donc construite autour d'une règle
simple : **un appel par tick**, et des données copiées une fois (registres en
JSON au chargement, sections de chunk à leur chargement).

## Installation (développeurs)

1. Ajoutez `rustlib-core` et `rustlib-jni` à votre crate de mod.
2. Exposez le mod à Java en une ligne :

   ```rust
   rustlib_jni::export_mod!(|| Box::new(MonMod::new()));
   ```

3. Compilez le jar (le script enchaîne Rust, javac, assemblage et vérification) :

   ```bash
   ./build-jar.sh     # → dist/rustlib-0.1.0.jar
   ```

4. Utilisez l'API Java depuis votre entrypoint Fabric :

   ```java
   RustLib lib = RustLib.start(2_000_000L);   // 2 ms de budget par tick
   String json = lib.registriesJson();        // blocs et items à créer
   lib.tick();                                // une fois par tick
   lib.blockBreak(x, y, z, "copper:copper_ore", "Alex");
   lib.close();
   ```

Le jar contient `dev/rustlib/RustLib.class`, un `fabric.mod.json` (bibliothèque
sans entrypoint) et la bibliothèque native de votre mod. Il se place dans
`mods/`, se déclare en jar imbriqué dans un mod Fabric, ou s'utilise hors
Minecraft pour tester la logique.

## Ce que RustLib ne fait pas

* **Pas de rendu, pas de client.** Aucun accès aux shaders, modèles, particules
  ou entrées : la cible est la logique serveur.
* **Pas de mixins depuis Rust.** Rust ne réécrit pas de bytecode Java : vos
  hooks passent par les API Fabric (ou par un mixin Java qui appelle le pont).
* **Pas de gain magique.** Déplacer du code en Rust n'accélère que ce qui était
  réellement coûteux et mal optimisé. Le calcul lourd sur des données déjà
  copiées gagne ; un accès ponctuel à l'API Minecraft perd.
* **Pas de persistance.** L'état Rust ne survit pas au redémarrage ; utilisez les
  structures de sauvegarde Minecraft côté Java.
* **Un jar par plateforme** : un `.dll` Windows ne se charge pas sur Linux.

## Un exemple fourni

Le dépôt livre **RustLib Mods**, un petit mod d'exemple : deux blocs, deux
items, un outil, et un quota de 64 casses de minerai par fenêtre de 100 ticks
qui montre un événement annulé par du code Rust. Toute sa logique est testée
sans serveur Minecraft.

## Qualité

* 58 tests automatisés (registres, priorités, annulation, budget de tick avec
  report, sections de chunk, JSON des registres).
* Un test de fumée qui charge le jar, extrait son natif, démarre le mod, envoie
  des ticks et vérifie une casse de bloc annulée — sans Minecraft.
* Erreurs explicites : identifiant invalide, doublon, registre gelé, namespace
  réservé.

## Statut

**Alpha 0.1.0.** L'API peut changer. L'intégration Fabric côté Java est
documentée et testée hors jeu (pont JNI compris) ; le branchement des événements
Fabric est à faire dans votre mod. Fourni pour Windows, Linux et macOS selon la
plateforme de compilation.

## Liens

* Wiki : 10 pages, de « premier mod » au pont JNI/Fabric et aux limites.
* Source : votre dépôt Git.

## Changelog 0.1.0

Première version publique.

* Noyau Rust sans dépendance : identifiants validés, registres typés avec gel,
  bus d'événements à priorités et annulation, ordonnanceur avec budget par tick,
  sections de chunk denses, export JSON des registres.
* Jar `rustlib.jar` : classe `dev.rustlib.RustLib`, `fabric.mod.json`, natif
  embarqué et chargé depuis le jar.
* Macro `export_mod!` : cinq symboles JNI à noms fixes, un jar pour tous les mods.
* Mod d'exemple RustLib Mods et 58 tests automatisés.

## Notes de publication

* Icône : 512 × 512 px, PNG (par exemple un engrenage sur fond fer).
* Galerie : capture du JSON des registres, extrait du wiki (page événements),
  schéma « un appel JNI par tick ».
* Rappel honnête : la fiche dit explicitement que c'est une bibliothèque pour
  développeurs, et que le branchement Fabric reste à faire dans le mod. Mieux
  vaut un mod sous-évalué qu'une promesse non tenue en jeu.
