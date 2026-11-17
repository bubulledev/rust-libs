# 5. Registres

Un registre associe un identifiant à une valeur typée, avec deux garanties :
**pas de doublon** et **gel après le chargement**.

```rust
use rustlib::prelude::*;

let mut blocks: Registry<Block> = Registry::new();
blocks.register(
    Block::builder(ResourceLocation::mc("monmod", "bore"))
        .material(Material::Metal)
        .hardness(4.0)
        .build()
        .unwrap(),
)?;

assert!(blocks.get(&ResourceLocation::mc("monmod", "bore")).is_some());
assert_eq!(blocks.len(), 1);
assert_eq!(blocks.ids().next().unwrap().path(), "bore");
```

Un type entre dans un registre dès qu'il implémente `RegistryEntry` :

```rust
pub trait RegistryEntry {
    fn id(&self) -> &ResourceLocation;
}
```

## Le gel

```rust
blocks.freeze();
assert_eq!(
    blocks.register(Block::builder(ResourceLocation::mc("monmod", "tard")).build().unwrap()),
    Err(RegistryError::Frozen)
);
```

Pourquoi geler ? Parce qu'un bloc enregistré après la génération du monde
produirait un monde que Minecraft ne sait plus relire. Le `Runtime` gèle ses
registres dans `finish_loading()` :

```rust
let mut runtime = Runtime::new(Duration::from_millis(2));
runtime.load(&mut MonMod::new())?;      // phase d'enregistrement
runtime.finish_loading();               // gel définitif
assert!(!runtime.is_loading());
```

## Le contenu standard

### Blocs

| Groupe | Méthode | Valeurs |
| --- | --- | --- |
| Matériau | `.material(Material)` | `Stone`, `Dirt`, `Wood`, `Metal`, `Glass`, `Plant`, `Misc` |
| Minage | `.hardness(f32)` | pierre `1.5`, obsidienne `50.0` |
| Explosions | `.resistance(f32)` | obsidienne `1200.0` |
| Lumière | `.light(u8)` | `0..=15`, validé |
| Transparence | `.transparent(bool)` | verre `true` |
| Butin | `.drops(iter)` | liste de `ResourceLocation` |

`Block::builder(id)` démarre avec `Material::Stone`, `hardness(1.0)`,
`resistance(1.0)`, `light(0)`, `transparent(false)` et aucun butin. Si `drops`
est vide, la JVM doit lâcher le bloc lui-même.

### Items

```rust
Item::builder(ResourceLocation::mc("monmod", "pioche"))
    .tool(Tool::new(ToolKind::Pickaxe, ToolTier::Diamond, 8.0, 3.0))
    .durability(1561)
    .build()
```

| Groupe | Méthode | Remarque |
| --- | --- | --- |
| Pile | `.stack_size(u8)` | `1..=64` |
| Durabilité | `.durability(u32)` | force la taille de pile à 1 |
| Outil | `.tool(Tool)` | force la taille de pile à 1 |
| Nourriture | `.food(u8)` | points de nourriture rendus |

`ToolTier::mining_level()` donne le niveau de minage (`0` bois/or … `4`
netherite), utilisé par la JVM pour décider quels blocs l'outil peut casser.

## L'export JSON

Une seule chaîne traverse la frontière, au chargement :

```rust
let json = runtime.registries_json();
// {"mods":[...],"blocks":[...],"items":[...]}
```

Exemple de bloc sérialisé :

```json
{
  "id": "copper:copper_ore",
  "material": "stone",
  "hardness": 3,
  "resistance": 3,
  "light": 0,
  "transparent": false,
  "drops": ["copper:raw_copper"]
}
```

Et un item-outil :

```json
{
  "id": "copper:copper_pickaxe",
  "max_stack": 1,
  "durability": 190,
  "tool": { "kind": "pickaxe", "tier": 1, "speed": 5, "attack_damage": 1 },
  "food_points": null
}
```

Le JSON est écrit par un sérialiseur maison (une centaine de lignes) pour que
le noyau n'ait aucune dépendance. Il échappe `"`, `\`, les sauts de ligne, les
tabulations et tout caractère de contrôle. Les champs `null` sont explicites :
côté Java, `durability` nul signifie « pas de durabilité ».

C'est la JVM qui crée ensuite les `Block`/`Item` Minecraft ; ce que RustLib
garantit, c'est que la description est complète, validée et cohérente.

## Ordre d'enregistrement

`Registry::iter()` et `Registry::ids()` suivent l'ordre d'enregistrement, pas
l'ordre alphabétique. Deux exécutions avec les mêmes mods produisent donc le
même JSON, octet pour octet — utile pour diffuser un manifeste reproductible.

→ [Événements](06-evenements.md)
