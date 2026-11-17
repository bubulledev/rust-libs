//! Bus d'événements typé.
//!
//! Les mods s'abonnent à des types d'événements concrets ; le bus les trie par
//! priorité et peut annuler un événement (casser un bloc, prendre des dégâts…).
//! Le tri est fait à l'abonnement, pas à la publication : publier un événement
//! ne coûte qu'une itération sur les gestionnaires concernés.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Un événement publié sur le bus.
///
/// L'annulation est optionnelle : les événements qui ne peuvent pas être
/// annulés laissent les implémentations par défaut.
pub trait Event: 'static {
    /// L'événement a-t-il été annulé par un mod ?
    fn is_cancelled(&self) -> bool {
        false
    }

    /// Annule l'événement (sans effet si l'événement n'est pas annulable).
    fn cancel(&mut self) {}
}

/// Priorité d'exécution d'un gestionnaire.
///
/// `Highest` s'exécute en premier, `Lowest` en dernier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventPriority {
    Lowest,
    Low,
    Normal,
    High,
    Highest,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Identifiant d'un abonnement, permettant de le retirer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HandlerId(u64);

type RawHandler = Box<dyn FnMut(&mut dyn Any) + Send>;

struct Handler {
    id: HandlerId,
    priority: EventPriority,
    receive_cancelled: bool,
    run: RawHandler,
}

/// Bus d'événements. Un seul bus par serveur, partagé par tous les mods.
#[derive(Default)]
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<Handler>>,
    next_id: u64,
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("event_types", &self.handlers.len())
            .field("handlers", &self.total_handlers())
            .finish()
    }
}


impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Abonne un gestionnaire, appelé uniquement si l'événement n'est pas
    /// encore annulé.
    pub fn subscribe<E: Event>(
        &mut self,
        priority: EventPriority,
        handler: impl FnMut(&mut E) + Send + 'static,
    ) -> HandlerId {
        self.insert(priority, false, handler)
    }

    /// Abonne un gestionnaire qui s'exécute même si l'événement est déjà
    /// annulé (utile pour observer, journaliser ou réagir à une annulation).
    pub fn subscribe_including_cancelled<E: Event>(
        &mut self,
        priority: EventPriority,
        handler: impl FnMut(&mut E) + Send + 'static,
    ) -> HandlerId {
        self.insert(priority, true, handler)
    }

    fn insert<E: Event>(
        &mut self,
        priority: EventPriority,
        receive_cancelled: bool,
        mut handler: impl FnMut(&mut E) + Send + 'static,
    ) -> HandlerId {
        let id = HandlerId(self.next_id);
        self.next_id += 1;

        let run: RawHandler = Box::new(move |any: &mut dyn Any| {
            if let Some(event) = any.downcast_mut::<E>() {
                handler(event);
            }
        });

        let list = self.handlers.entry(TypeId::of::<E>()).or_default();
        // Les identifiants croissent et les priorités décroissent : insérer au
        // premier gestionnaire de priorité strictement inférieure suffit à
        // garder la liste ordonnée (priorité puis ordre d'abonnement).
        let position = list
            .iter()
            .position(|h| h.priority < priority)
            .unwrap_or(list.len());
        list.insert(
            position,
            Handler {
                id,
                priority,
                receive_cancelled,
                run,
            },
        );

        id
    }

    /// Publie un événement.
    ///
    /// Renvoie `true` si l'événement n'a pas été annulé (il peut donc se
    /// produire), `false` s'il a été annulé.
    pub fn post<E: Event>(&mut self, event: &mut E) -> bool {
        if let Some(list) = self.handlers.get_mut(&TypeId::of::<E>()) {
            for handler in list.iter_mut() {
                if event.is_cancelled() && !handler.receive_cancelled {
                    continue;
                }
                (handler.run)(event as &mut dyn Any);
            }
        }
        !event.is_cancelled()
    }

    /// Retire un abonnement. Renvoie `false` s'il n'existe plus.
    pub fn unsubscribe<E: Event>(&mut self, id: HandlerId) -> bool {
        let Some(list) = self.handlers.get_mut(&TypeId::of::<E>()) else {
            return false;
        };
        let before = list.len();
        list.retain(|h| h.id != id);
        list.len() != before
    }

    /// Nombre d'abonnements pour un type d'événement.
    pub fn handler_count<E: Event>(&self) -> usize {
        self.handlers
            .get(&TypeId::of::<E>())
            .map_or(0, |list| list.len())
    }

    /// Nombre total d'abonnements, tous types confondus.
    pub fn total_handlers(&self) -> usize {
        self.handlers.values().map(|list| list.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Ping {
        cancelled: bool,
    }

    impl Event for Ping {
        fn is_cancelled(&self) -> bool {
            self.cancelled
        }
        fn cancel(&mut self) {
            self.cancelled = true;
        }
    }

    struct Pong;
    impl Event for Pong {}

    fn recorder() -> Arc<Mutex<Vec<&'static str>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn priorites_respectees() {
        let log = recorder();
        let mut bus = EventBus::new();
        for (priority, tag) in [
            (EventPriority::Lowest, "lowest"),
            (EventPriority::Normal, "normal"),
            (EventPriority::Highest, "highest"),
        ] {
            let log = log.clone();
            bus.subscribe::<Ping>(priority, move |_| log.lock().unwrap().push(tag));
        }
        let mut event = Ping::default();
        assert!(bus.post(&mut event));
        assert_eq!(*log.lock().unwrap(), ["highest", "normal", "lowest"]);
    }

    #[test]
    fn ordre_d_abonnement_a_priorite_egale() {
        let log = recorder();
        let mut bus = EventBus::new();
        for tag in ["premier", "second", "troisieme"] {
            let log = log.clone();
            bus.subscribe::<Ping>(EventPriority::Normal, move |_| log.lock().unwrap().push(tag));
        }
        bus.post(&mut Ping::default());
        assert_eq!(*log.lock().unwrap(), ["premier", "second", "troisieme"]);
    }

    #[test]
    fn annulation_bloque_les_gestionnaires_suivants() {
        let log = recorder();
        let mut bus = EventBus::new();
        let first = log.clone();
        bus.subscribe::<Ping>(EventPriority::Highest, move |event| {
            first.lock().unwrap().push("annule");
            event.cancel();
        });
        let second = log.clone();
        bus.subscribe::<Ping>(EventPriority::Normal, move |_| {
            second.lock().unwrap().push("jamais");
        });

        let mut event = Ping::default();
        assert!(!bus.post(&mut event));
        assert_eq!(*log.lock().unwrap(), ["annule"]);
    }

    #[test]
    fn abonnement_aux_evenements_annules() {
        let log = recorder();
        let mut bus = EventBus::new();
        let a = log.clone();
        bus.subscribe::<Ping>(EventPriority::Highest, move |e| {
            a.lock().unwrap().push("cancel");
            e.cancel();
        });
        let b = log.clone();
        bus.subscribe_including_cancelled::<Ping>(EventPriority::Lowest, move |e| {
            b.lock().unwrap().push(if e.is_cancelled() { "observateur" } else { "?" });
        });

        bus.post(&mut Ping::default());
        assert_eq!(*log.lock().unwrap(), ["cancel", "observateur"]);
    }

    #[test]
    fn types_d_evenements_isoles() {
        let log = recorder();
        let mut bus = EventBus::new();
        let ping = log.clone();
        bus.subscribe::<Ping>(EventPriority::Normal, move |_| {
            ping.lock().unwrap().push("ping")
        });
        let pong = log.clone();
        bus.subscribe::<Pong>(EventPriority::Normal, move |_| {
            pong.lock().unwrap().push("pong")
        });

        bus.post(&mut Pong);
        assert_eq!(*log.lock().unwrap(), ["pong"]);
        assert_eq!(bus.handler_count::<Ping>(), 1);
        assert_eq!(bus.total_handlers(), 2);
    }

    #[test]
    fn desabonnement() {
        let log = recorder();
        let mut bus = EventBus::new();
        let l = log.clone();
        let id = bus.subscribe::<Ping>(EventPriority::Normal, move |_| {
            l.lock().unwrap().push("appele")
        });
        assert!(bus.unsubscribe::<Ping>(id));
        assert!(!bus.unsubscribe::<Ping>(id));
        bus.post(&mut Ping::default());
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn evenement_non_annulable_reste_vrai() {
        let mut bus = EventBus::new();
        bus.subscribe::<Pong>(EventPriority::Normal, |_| {});
        assert!(bus.post(&mut Pong));
    }
}
