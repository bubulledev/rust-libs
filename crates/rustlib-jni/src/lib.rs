//! Pont JNI entre un mod Fabric et RustLib.
//!
//! Le mod Java charge la bibliothèque dynamique produite par votre crate Rust,
//! puis appelle quelques méthodes `native`. Chaque appel est traduit ici en
//! opération Rust, sans logique métier.
//!
//! # Sens des échanges
//!
//! * **Java → Rust** : des entiers, sauf pour la casse de bloc qui transporte
//!   deux identifiants. Aucun objet Java n'est conservé côté Rust.
//! * **Rust → Java** : une seule chaîne JSON, au chargement, décrivant les
//!   registres à créer ([`registries_json`]).
//!
//! Ce découpage est volontaire : une frontière JNI traversée vingt fois par tick
//! coûte plus cher que la même logique en Java. Le Rust reçoit **un** appel par
//! tick et fait le reste chez lui.
//!
//! # Cycle de vie
//!
//! ```text
//! nativeCreate(budgetNanos)  -> handle (0 = échec, exception levée)
//! nativeRegistries(handle)   -> JSON des blocs et items à créer
//! nativeTick(handle)         -> tâches Rust exécutées pendant ce tick
//! nativeBlockBreak(...)      -> false si un mod annule la casse
//! nativeDestroy(handle)      -> libération
//! ```
//!
//! Le handle est un `Box<Runtime>` brut : le côté Java est le seul propriétaire,
//! et `nativeDestroy` est le seul point de libération. L'instance du mod, elle,
//! n'est conservée que le temps du chargement — son état vit ensuite dans ses
//! gestionnaires (`Arc`) et dans le [`Runtime`].
//!
//! Le côté Java de ce contrat est décrit dans `wiki/08-pont-jni-fabric.md`, et
//! la macro [`export_mod`] génère les cinq symboles attendus par la classe Java
//! `dev.rustlib.RustLib`.

use std::time::Duration;

use jni::objects::JString;
use jni::sys::{jboolean, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use rustlib::events::{BlockBreak, ServerTick};
use rustlib::math::BlockPos;
use rustlib::prelude::*;

/// Ré-export de `jni` pour la macro [`export_mod`].
pub use ::jni;
/// Ré-export du noyau pour la macro [`export_mod`].
pub use ::rustlib;

/// Charge le mod et renvoie un handle opaque pour la JVM.
///
/// # Errors
/// Renvoie un message si le chargement échoue (métadonnées invalides, doublon
/// d'identifiant, registre déjà gelé).
pub fn create(module: Box<dyn Mod>, tick_budget_nanos: i64) -> Result<jlong, String> {
    let budget = Duration::from_nanos(tick_budget_nanos.max(0) as u64);
    let mut runtime = Runtime::new(budget);
    let mut module = module;

    runtime
        .load(module.as_mut())
        .map_err(|err| format!("échec du chargement du mod : {err}"))?;
    runtime.finish_loading();

    Ok(Box::into_raw(Box::new(runtime)) as jlong)
}

/// Libère un handle créé par [`create`]. Sans effet si `handle` vaut 0.
///
/// # Safety
/// `handle` doit provenir de [`create`] et ne pas être utilisé après l'appel.
pub fn destroy(handle: jlong) {
    if handle == 0 {
        return;
    }
    // SAFETY: contrat de la fonction, voir ci-dessus.
    unsafe {
        drop(Box::from_raw(handle as *mut Runtime));
    }
}

/// Emprunte le runtime derrière un handle.
///
/// # Safety
/// `handle` doit provenir de [`create`] et ne pas avoir été détruit. Le handle
/// est créé et libéré par la JVM, d'où la présence d'`unsafe` à l'intérieur de
/// ce module et nulle part ailleurs.
unsafe fn runtime_mut<'a>(handle: jlong) -> Option<&'a mut Runtime> {
    if handle == 0 {
        None
    } else {
        Some(&mut *(handle as *mut Runtime))
    }
}

/// Exécute un tick côté Rust et publie [`ServerTick`].
///
/// Renvoie le nombre de tâches exécutées, ou -1 si le handle est invalide.
pub fn tick(handle: jlong) -> jlong {
    // SAFETY: handle produit par `create`, voir sa documentation.
    let Some(runtime) = (unsafe { runtime_mut(handle) }) else {
        return -1;
    };
    let report = runtime.tick();
    let current = runtime.current_tick();
    runtime.post(&mut ServerTick { tick: current });
    report.executed as jlong
}

/// Renvoie le JSON des registres à créer côté Java, ou `null` si le handle est
/// invalide.
pub fn registries_json(env: &mut JNIEnv, handle: jlong) -> jstring {
    // SAFETY: handle produit par `create`.
    let Some(runtime) = (unsafe { runtime_mut(handle) }) else {
        return std::ptr::null_mut();
    };
    let json = runtime.registries_json();
    match env.new_string(json) {
        Ok(string) => string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Signale une casse de bloc : renvoie `true` si la casse est autorisée.
///
/// Un mod peut annuler l'événement ([`BlockBreak`]), auquel cas la JVM doit
/// annuler la casse et remettre le bloc en place. Les entrées illisibles ou
/// d'identifiant invalide n'annulent jamais la casse.
pub fn on_block_break(
    env: &mut JNIEnv,
    handle: jlong,
    x: jint,
    y: jint,
    z: jint,
    block: &JString,
    player: &JString,
) -> jboolean {
    let (Ok(block), Ok(player)) = (env.get_string(block), env.get_string(player)) else {
        return JNI_TRUE;
    };
    let block_name = String::from(block);
    let player_name = String::from(player);
    let Ok(block) = ResourceLocation::parse(&block_name) else {
        return JNI_TRUE;
    };

    let mut event = BlockBreak::new(block, BlockPos::new(x, y, z), player_name);
    // SAFETY: handle produit par `create`.
    let allowed = match unsafe { runtime_mut(handle) } {
        Some(runtime) => runtime.post(&mut event),
        None => true,
    };

    if allowed {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// Lève une `IllegalStateException` côté Java, pour signaler une erreur du pont
/// sans faire tomber la JVM.
pub fn raise(env: &mut JNIEnv, message: &str) {
    let _ = env.throw_new("java/lang/IllegalStateException", message);
}

/// Exporte les cinq symboles JNI attendus par la classe Java
/// `dev.rustlib.RustLib`, livrée dans `rustlib.jar`.
///
/// Les noms de symboles sont fixes : le même jar fonctionne avec la bibliothèque
/// dynamique de n'importe quel mod.
///
/// ```ignore
/// rustlib_jni::export_mod!(|| Box::new(MonMod::new()));
/// ```
#[macro_export]
macro_rules! export_mod {
    ($factory:expr) => {
        #[no_mangle]
        pub extern "system" fn Java_dev_rustlib_RustLib_nativeCreate(
            mut env: $crate::jni::JNIEnv,
            _class: $crate::jni::objects::JClass,
            budget_nanos: $crate::jni::sys::jlong,
        ) -> $crate::jni::sys::jlong {
            let module: Box<dyn $crate::rustlib::Mod> = ($factory)();
            match $crate::create(module, budget_nanos) {
                Ok(handle) => handle,
                Err(message) => {
                    // L'exception levée ici est celle que voit le code Java.
                    $crate::raise(&mut env, &message);
                    0
                }
            }
        }

        #[no_mangle]
        pub extern "system" fn Java_dev_rustlib_RustLib_nativeRegistries(
            mut env: $crate::jni::JNIEnv,
            _class: $crate::jni::objects::JClass,
            handle: $crate::jni::sys::jlong,
        ) -> $crate::jni::sys::jstring {
            $crate::registries_json(&mut env, handle)
        }

        #[no_mangle]
        pub extern "system" fn Java_dev_rustlib_RustLib_nativeTick(
            _env: $crate::jni::JNIEnv,
            _class: $crate::jni::objects::JClass,
            handle: $crate::jni::sys::jlong,
        ) -> $crate::jni::sys::jlong {
            $crate::tick(handle)
        }

        #[no_mangle]
        pub extern "system" fn Java_dev_rustlib_RustLib_nativeBlockBreak(
            mut env: $crate::jni::JNIEnv,
            _class: $crate::jni::objects::JClass,
            handle: $crate::jni::sys::jlong,
            x: $crate::jni::sys::jint,
            y: $crate::jni::sys::jint,
            z: $crate::jni::sys::jint,
            block: $crate::jni::objects::JString,
            player: $crate::jni::objects::JString,
        ) -> $crate::jni::sys::jboolean {
            $crate::on_block_break(&mut env, handle, x, y, z, &block, &player)
        }

        #[no_mangle]
        pub extern "system" fn Java_dev_rustlib_RustLib_nativeDestroy(
            _env: $crate::jni::JNIEnv,
            _class: $crate::jni::objects::JClass,
            handle: $crate::jni::sys::jlong,
        ) {
            $crate::destroy(handle)
        }
    };
}
