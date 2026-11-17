# 10. Limites et FAQ

## Limites assumées

### Le rendu et le client sont hors périmètre

RustLib ne touche pas au rendu, aux modèles, aux particules, aux touches ni à
l'interface. La frontière JNI d'un client graphique (OpenGL) est instable et
chaque image serait un cauchemar de synchronisation. Si votre mod est
principalement visuel, restez en Java.

### Rust ne remplace pas les mixins

Un mixin réécrit du bytecode Java ; Rust ne peut pas le faire. Le pont se
branche **sur les API Fabric** (`ServerTickEvents`, `PlayerBlockBreakEvents`,
`ServerLifecycleEvents`…). Si l'API n'expose pas ce que vous cherchez, vous
devrez soit écrire un mixin Java qui appelle le pont, soit renoncer.

### Un appel JNI coûte cher

Voir [Ticks et performance](07-ticks-et-performance.md). Un appel par tick, pas
un par bloc. La bibliothèque est conçue autour de cette contrainte ; la
contourner annule le bénéfice.

### Une bibliothèque par plateforme

Un `.dll` Windows ne se charge pas sur Linux. Distribuer un mod avec du code
natif implique un artefact par OS et par architecture (x86-64, aarch64). C'est
la principale raison pour laquelle si peu de mods Minecraft embarquent du natif.

### L'état Rust ne survit pas au redémarrage

Les registres, le cache de sections et les compteurs vivent en mémoire. Si vous
avez besoin de persistance, écrivez dans les structures Minecraft (NBT,
`SavedData`) côté Java, qui sont sauvegardées par le jeu.

### Pas de `Runtime` partagé entre fils

`Runtime` est `Send` mais pas `Sync` : un seul fil doit le piloter, en pratique
le fil du serveur. Le pont n'ajoute aucun verrou global, ce qui est un choix de
performance, mais vous devez respecter cette règle d'appel.

## FAQ

### Est-ce que mon mod sera plus rapide ?

Seulement si la partie déplacée en Rust est un calcul volumineux sur des données
déjà copiées. Un mod qui passe son temps à interroger l'API Minecraft via la
frontière sera plus lent. Mesurez, comparez, décidez.

### Pourquoi aucune dépendance dans `rustlib-core` ?

Pour que rien ne s'interpose entre votre code et le compilateur : pas de
sérialiseur à configurer, pas de runtime asynchrone dans le tick, pas de
bibliothèque mathématique aux conventions surprenantes. Le coût est un
sérialiseur JSON maison d'une centaine de lignes, entièrement testé.

### Pourquoi les registres se gèlent-ils ?

Parce qu'un bloc enregistré après la génération du monde produit un monde que
Minecraft ne sait plus relire. Le gel transforme un bug de synchronisation en
erreur immédiate et explicite (`RegistryError::Frozen`).

### Pourquoi le budget de tick plutôt qu'un simple compteur de tâches ?

Un compteur ne dit rien du temps réel consommé : dix tâches rapides valent mieux
qu'une seule tâche lente. Le budget mesure le temps effectivement passé et
reporte le reste, ce qui garde le TPS stable même quand la charge varie.

### Comment déboguer un mod Rust chargé par la JVM ?

* Activé par défaut : les `panic` sont fatals (`panic = "abort"`). Pour un
  diagnostic, compilez temporairement sans `panic = "abort"` en local et
  attachez un débogueur au processus Java (`rust-lldb`, `cdb`).
* Les journaux Java affichent les exceptions levées par `raise`.
* Le plus efficace reste le test hors JVM : `Runtime` + `post` + `tick` couvrent
  la quasi-totalité de la logique (voir [Premier mod](03-premier-mod.md)).

### Puis-je avoir plusieurs mods Rust dans un même processus ?

Oui : chaque mod a son propre handle et son propre `Runtime`. Deux mods Rust ne
partagent pas le bus d'événements entre eux — c'est une limite actuelle. Si vous
avez besoin d'événements inter-mods, chargez plusieurs `Mod` dans un même
`Runtime` :

```rust
let mut runtime = Runtime::new(Duration::from_millis(2));
runtime.load(&mut ModA::new())?;
runtime.load(&mut ModB::new())?;   // bus et registres partagés
runtime.finish_loading();
```

### Comment contribuer ou signaler un problème ?

Les tests sont la spécification : `cargo test --workspace` doit passer, et tout
changement de comportement doit venir avec son test. Le JSON des registres est
couvert pour éviter les régressions de format, qui casseraient la compatibilité
avec la partie Java.

→ [Retour au sommaire](README.md)
