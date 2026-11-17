# 8. Pont JNI, jar et Fabric

Cette page décrit ce qui se passe entre le serveur Minecraft et votre mod Rust :
le jar `rustlib.jar`, la classe Java `dev.rustlib.RustLib`, la macro
`export_mod!`, puis le branchement Fabric. **Le code Java du jar est compilé et
testé** (`./build-jar.sh`), **le branchement Fabric est un modèle** : il dépend
de votre version de l'API et n'est pas compilé ici.

## Le jar `rustlib.jar`

```bash
./build-jar.sh
```

Produit `dist/rustlib-0.1.0.jar` :

```text
META-INF/MANIFEST.MF          Implementation-Title: RustLib
fabric.mod.json               id "rustlib", sans entrypoint (bibliothèque)
dev/rustlib/RustLib.class     l'API Java
native/rustlib.dll            la bibliothèque native du mod compilé
                              (librustlib.so / librustlib.dylib sur Linux / macOS)
```

Trois usages possibles :

1. **Dans un mod Fabric** : copiez le contenu du jar dans le jar de votre mod
   (ou déclarez-le en `jars` dans `fabric.mod.json` pour un chargement imbriqué
   — Fabric appelle cela *Jar-in-Jar*).
2. **Dans `mods/`** : le jar contient un `fabric.mod.json` sans entrypoint ;
   Fabric l'ajoute au classpath comme mod bibliothèque.
3. **Hors Minecraft** : `java -cp dist/rustlib-0.1.0.jar` et un `main` qui
   appelle `RustLib.start()` — c'est exactement ce que fait le test de fumée.

Le nom du fichier natif dépend de la plateforme : un jar compilé sous Windows
n'embarque que `rustlib.dll`. Distribuez un jar par plateforme, ou compilez dans
une CI multi-plateforme.

## L'API Java

`dev.rustlib.RustLib` est une façade `AutoCloseable` :

| Méthode | Rôle |
| --- | --- |
| `RustLib.start()` / `start(long budgetNanos)` | extrait le natif du jar, le charge et démarre le mod (2 ms par défaut) |
| `registriesJson()` | JSON des blocs et items à créer |
| `tick()` | exécute un tick Rust, renvoie le nombre de tâches exécutées |
| `blockBreak(x, y, z, bloc, joueur)` | `false` = casse annulée par un mod |
| `close()` | arrêt et libération côté Rust (idempotent) |

Un seul chemin de chargement : le natif vient toujours du jar. `start()` est donc
le point d'entrée unique. Après `close()`, les appels lèvent une
`IllegalStateException` au lieu de repasser un handle libéré à la frontière JNI.

## Côté Rust : une seule ligne

La macro `rustlib_jni::export_mod!` génère les cinq symboles JNI attendus par la
classe Java, avec des noms **fixes** : le même jar fonctionne avec la
bibliothèque de n'importe quel mod.

```rust
// examples/rustlib-mods/src/lib.rs
rustlib_jni::export_mod!(|| Box::new(RustLibMods::new()));
```

Ce que la macro produit, en substance :

```text
Java_dev_rustlib_RustLib_nativeCreate    → rustlib_jni::create
Java_dev_rustlib_RustLib_nativeRegistries→ rustlib_jni::registries_json
Java_dev_rustlib_RustLib_nativeTick      → rustlib_jni::tick
Java_dev_rustlib_RustLib_nativeBlockBreak→ rustlib_jni::on_block_break
Java_dev_rustlib_RustLib_nativeDestroy   → rustlib_jni::destroy
```

Si vous préférez écrire les symboles à la main (autre nom de classe Java, autre
paquet), respectez la convention
`Java_<paquet>_<Classe>_<méthode>` avec des points remplacés par des `_`,
`extern "system"` et `#[no_mangle]`, et **aucune logique métier** dans ces
fonctions.

## Brancher Fabric

Modèle d'entrypoint (à adapter à votre version) :

```java
public final class CopperFabric implements ModInitializer {
    private static RustLib lib;

    @Override
    public void onInitialize() {
        lib = RustLib.start(2_000_000L);            // 2 ms par tick

        String json = lib.registriesJson();
        registerContent(json);                      // voir section suivante

        ServerTickEvents.END_SERVER_TICK.register(server -> lib.tick());

        PlayerBlockBreakEvents.BEFORE.register((world, player, pos, state, entity) ->
                lib.blockBreak(pos.getX(), pos.getY(), pos.getZ(),
                        Registries.BLOCK.getId(state.getBlock()).toString(),
                        player.getName().getString()));

        ServerLifecycleEvents.SERVER_STOPPED.register(server -> lib.close());
    }
}
```

Points d'attention :

* `PlayerBlockBreakEvents.BEFORE` renvoie un `boolean` : c'est là que le
  `false` de `blockBreak` annule la casse.
* Appelez toujours le pont depuis le **fil du serveur**. `Runtime` est `Send`
  mais pas `Sync` : un seul fil doit le piloter.
* Les noms d'événements Fabric changent selon la version de l'API — vérifiez les
  signatures de la vôtre.
* `Registries.BLOCK.getId(...)` renvoie `namespace:path`, exactement le format
  attendu par `ResourceLocation::parse`.

## Transformer le JSON en contenu Minecraft

Le JSON décrit vos blocs et items ; la création effective dépend de votre
version. Modèle de lecture (Gson est fourni par Minecraft) :

```java
JsonObject root = JsonParser.parseString(json).getAsJsonObject();

for (JsonElement element : root.getAsJsonArray("blocks")) {
    JsonObject block = element.getAsJsonObject();
    Identifier id = Identifier.of(block.get("id").getAsString());
    float hardness = block.get("hardness").getAsFloat();
    int light = block.get("light").getAsInt();
    // Construire un BlockSettings équivalent puis l'enregistrer
    // (Registries.BLOCK) selon l'API de votre version.
}
```

Champs disponibles dans [Registres](05-registres.md) : `id`, `material`,
`hardness`, `resistance`, `light`, `transparent`, `drops` pour les blocs ;
`id`, `max_stack`, `durability`, `tool{kind, tier, speed, attack_damage}`,
`food_points` pour les items.

Deux stratégies, selon votre version :

1. **Enregistrement dynamique** des blocs décrits par le JSON, à
   l'initialisation du mod.
2. **Registres statiques en Java** (une classe par bloc) et JSON utilisé comme
   **validation** : le pont vérifie que Java et Rust déclarent la même chose.
   Plus verbeux, mais compatible avec toutes les versions et les datapacks.

## Dépannage

| Erreur | Cause probable |
| --- | --- |
| `UnsatisfiedLinkError: no rustlib in java.library.path` | natif absent du jar, ou fichier natif corrompu |
| `UnsatisfiedLinkError: ... method not found` | symboles absents : le natif n'a pas été compilé avec `export_mod!` |
| `IllegalStateException : /native/rustlib.dll absent du jar` | jar compilé pour une autre plateforme |
| `IllegalStateException : chargement refusé` | métadonnées invalides, mod déjà chargé, registre gelé |
| Le serveur plante au tick | `panic` dans une tâche Rust : `panic = "abort"` et corrigez la cause |
| La casse de bloc n'est jamais annulée | `PlayerBlockBreakEvents.BEFORE` non branché |

→ [Référence API](09-reference-api.md)
