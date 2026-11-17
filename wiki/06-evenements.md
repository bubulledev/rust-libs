# 6. Événements

Le bus est typé par le type d'événement, ordonné par priorité, et sait annuler.

## Déclarer un événement

```rust
use rustlib::prelude::*;

struct MonEvenement {
    pub valeur: u32,
    cancelled: bool,
}

impl Event for MonEvenement {
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    fn cancel(&mut self) {
        self.cancelled = true;
    }
}
```

Les deux méthodes ont des implémentations par défaut (`false` et « ne rien
faire »). Un événement purement informatif s'écrit donc `impl Event for MonEvenement {}`.

## S'abonner

```rust
let id = bus.subscribe::<MonEvenement>(EventPriority::High, |event| {
    if event.valeur > 10 {
        event.cancel();
    }
});
```

| Priorité | Usage typique |
| --- | --- |
| `Highest` | Sécurité : bloquer avant tout le monde |
| `High` | Règles de jeu propres au mod |
| `Normal` | Cas général |
| `Low` | Ajustements, compatibilité avec d'autres mods |
| `Lowest` | Journalisation, statistiques |

Ordre d'exécution : `Highest` → `Lowest`, puis ordre d'abonnement à priorité
égale. Cet ordre est stable et déterministe : le tri est fait à l'abonnement, pas
à la publication.

## Annulation

```rust
let mut event = MonEvenement { valeur: 42, cancelled: false };
let autorise = bus.post(&mut event);   // false : annulé
```

`post` renvoie `true` si l'événement peut se produire. Après une annulation, les
gestionnaires restants sont **ignorés** — sauf ceux abonnés avec
`subscribe_including_cancelled`, prévus pour observer :

```rust
bus.subscribe_including_cancelled::<MonEvenement>(EventPriority::Lowest, |event| {
    if event.is_cancelled() {
        // comptabiliser une action refusée, par exemple
    }
});
```

Le pont JNI utilise exactement ce mécanisme : `nativeBlockBreak` publie un
`BlockBreak` et renvoie la valeur de `post` à la JVM, qui annule la casse si la
réponse est `false`.

## Se désabonner

```rust
assert!(bus.unsubscribe::<MonEvenement>(id));
```

`unsubscribe` prend le type d'événement : le même `HandlerId` peut exister pour
plusieurs types sans ambiguïté de comptage.

## Événements standard

| Type | Champs | Annulable |
| --- | --- | --- |
| `BlockBreak` | `block`, `pos`, `player` | oui |
| `BlockPlace` | `block`, `pos`, `player` | oui |
| `PlayerDamage` | `player`, `amount` | oui |
| `ServerTick` | `tick` | non |
| `PlayerJoin` | `player` | non |
| `PlayerQuit` | `player` | non |

```rust
use rustlib::events::PlayerDamage;

runtime.events_mut().subscribe::<PlayerDamage>(EventPriority::Highest, |event| {
    if event.player == "invite" {
        event.amount *= 0.5;      // réduction des dégâts
    }
});
```

## Règles pratiques

1. **Un gestionnaire reste court.** Il s'exécute dans le tick du serveur. Pour
   un calcul long, poussez le travail dans `scheduler_mut()` et laissez le
   budget répartir la charge sur plusieurs ticks.
2. **Pas de blocage.** Un `Mutex` tenu par un autre fil dans un gestionnaire
   bloque le serveur : préférez les atomiques (`AtomicU64`) ou un canal.
3. **Partagez l'état par `Arc`.** Les closures sont `Send + 'static` ; elles ne
   peuvent pas emprunter le mod. C'est voulu : l'état partagé est explicite.
4. **Ne dépendez pas de l'ordre entre mods** à priorité égale. Si votre règle
   doit passer avant une autre, choisissez une priorité plus haute.

## Ce qui n'est pas fourni

* Pas d'événements client (rendu, touches) : le pont ne couvre que le serveur.
* Pas de priorité arbitraire (un `f32`) : cinq niveaux suffisent et rendent
  l'ordre lisible dans les journaux.
* Pas de propagation d'exception entre gestionnaires : une closure Rust ne
  renvoie rien. Journalisez ou stockez l'erreur dans un état partagé.

→ [Ticks et performance](07-ticks-et-performance.md)
