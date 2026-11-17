//! Événements standard du serveur.
//!
//! Ces types sont partagés par tous les mods chargés : le pont JNI les publie
//! quand la JVM signale l'action correspondante (casse de bloc, connexion,
//! dégâts…). Les types marqués `cancelled` sont annulables : annuler un
//! [`BlockBreak`] empêche la casse côté Minecraft.

use crate::event::Event;
use crate::math::BlockPos;

macro_rules! cancellable {
    ($ty:ty) => {
        impl Event for $ty {
            fn is_cancelled(&self) -> bool {
                self.cancelled
            }

            fn cancel(&mut self) {
                self.cancelled = true;
            }
        }
    };
}

macro_rules! informational {
    ($ty:ty) => {
        impl Event for $ty {}
    };
}

/// Un joueur casse un bloc. Annulable.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockBreak {
    /// Identifiant de l'état ou du bloc cassé.
    pub block: crate::id::ResourceLocation,
    /// Position du bloc.
    pub pos: BlockPos,
    /// Nom du joueur à l'origine de la casse.
    pub player: String,
    pub cancelled: bool,
}

impl BlockBreak {
    pub fn new(block: crate::id::ResourceLocation, pos: BlockPos, player: impl Into<String>) -> Self {
        Self {
            block,
            pos,
            player: player.into(),
            cancelled: false,
        }
    }
}

cancellable!(BlockBreak);

/// Un joueur place un bloc. Annulable.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockPlace {
    pub block: crate::id::ResourceLocation,
    pub pos: BlockPos,
    pub player: String,
    pub cancelled: bool,
}

impl BlockPlace {
    pub fn new(block: crate::id::ResourceLocation, pos: BlockPos, player: impl Into<String>) -> Self {
        Self {
            block,
            pos,
            player: player.into(),
            cancelled: false,
        }
    }
}

cancellable!(BlockPlace);

/// Un joueur va subir des dégâts. Annulable.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDamage {
    pub player: String,
    pub amount: f32,
    pub cancelled: bool,
}

impl PlayerDamage {
    pub fn new(player: impl Into<String>, amount: f32) -> Self {
        Self {
            player: player.into(),
            amount,
            cancelled: false,
        }
    }
}

cancellable!(PlayerDamage);

/// Un tick du serveur vient de commencer. Non annulable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerTick {
    pub tick: u64,
}

informational!(ServerTick);

/// Un joueur rejoint le serveur. Non annulable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerJoin {
    pub player: String,
}

informational!(PlayerJoin);

/// Un joueur quitte le serveur. Non annulable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuit {
    pub player: String,
}

informational!(PlayerQuit);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBus, EventPriority};
    use crate::id::ResourceLocation;

    #[test]
    fn casse_annulable() {
        let mut bus = EventBus::new();
        bus.subscribe::<BlockBreak>(EventPriority::Normal, |event| {
            if event.player == "tricheur" {
                event.cancel();
            }
        });

        let mut legit = BlockBreak::new(
            ResourceLocation::parse("minecraft:stone").unwrap(),
            BlockPos::new(0, 64, 0),
            "joueur",
        );
        assert!(bus.post(&mut legit));

        let mut triche = BlockBreak::new(
            ResourceLocation::parse("minecraft:stone").unwrap(),
            BlockPos::new(0, 64, 0),
            "tricheur",
        );
        assert!(!bus.post(&mut triche));
    }

    #[test]
    fn evenements_informatifs() {
        let mut bus = EventBus::new();
        bus.subscribe::<PlayerJoin>(EventPriority::Lowest, |event| {
            assert_eq!(event.player, "Alex");
        });
        assert!(bus.post(&mut PlayerJoin {
            player: "Alex".to_string()
        }));
        assert!(bus.post(&mut ServerTick { tick: 1 }));
    }
}
