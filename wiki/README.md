# Wiki RustLib

RustLib permet d'écrire la partie logique et gourmande de vos mods Minecraft en
Rust, tout en gardant un mod Fabric Java classique comme point d'entrée.

## Sommaire

1. [Introduction](01-introduction.md) — ce que RustLib fait, ce qu'il ne fait pas
2. [Installation](02-installation.md) — outils, compilation, artefacts
3. [Premier mod](03-premier-mod.md) — un bloc, un item, un événement, une tâche
4. [Architecture](04-architecture.md) — les crates, le flux d'un tick
5. [Registres](05-registres.md) — blocs, items, gel, export JSON
6. [Événements](06-evenements.md) — priorités, annulation, événements standard
7. [Ticks et performance](07-ticks-et-performance.md) — budget de tick et gain réel
8. [Pont JNI / Fabric](08-pont-jni-fabric.md) — symboles et code Java
9. [Référence API](09-reference-api.md) — toutes les signatures
10. [Limites et FAQ](10-limites-faq.md) — questions fréquentes

## Par où commencer ?

* Vous voulez juste voir du code tourner : [Premier mod](03-premier-mod.md).
* Vous voulez comprendre le découpage Java/Rust : [Architecture](04-architecture.md).
* Vous vous demandez si le Rust va vraiment accélérer votre mod :
  [Ticks et performance](07-ticks-et-performance.md).

## Vérifier votre copie du dépôt

```bash
cargo test --workspace     # 58 tests : registres, événements, budget, chunks, JSON
./build-jar.sh             # dist/rustlib-0.1.0.jar + vérification jar → JNI → Rust
```

Tout ce qui est décrit ici est vérifiable dans le dépôt : le noyau
(`crates/rustlib-core`), le pont (`crates/rustlib-jni`), le mod d'exemple
(`examples/rustlib-mods`) et le jar (`java/`, `dist/rustlib-0.1.0.jar`).
