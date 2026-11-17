package dev.rustlib.fabric;

import dev.rustlib.RustLib;
import java.util.Locale;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Entrypoint Fabric du mod RustLib Mods.
 *
 * <p>Le mod n'a que trois responsabilités côté Java : démarrer le runtime Rust,
 * lui envoyer un tick par tick de serveur, et l'arrêter quand le serveur
 * s'arrête. Toute la logique (registres, événements, quota de casse) vit dans
 * la crate Rust {@code examples/rustlib-mods}, empaquetée sous
 * {@code native/rustlib_mods.dll}.
 */
public final class RustLibMods implements ModInitializer {

    /** Identifiant du mod, identique à celui de fabric.mod.json. */
    public static final String MOD_ID = "rustlib_mods";

    private static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);

    /** Runtime Rust ; {@code null} tant que le natif n'est pas chargé. */
    private RustLib lib;

    /** Nombre de ticks Rust déjà exécutés (sert uniquement au log du premier). */
    private long ticks;

    @Override
    public void onInitialize() {
        // Un tick de serveur = un tick Rust (serveur dédié comme serveur intégré).
        ServerTickEvents.END_SERVER_TICK.register(server -> tick());
        ServerLifecycleEvents.SERVER_STOPPED.register(server -> stop());
        start();
    }

    /**
     * Démarre RustLib. Un natif absent (autre plateforme que celle compilée) ou
     * un chargement en erreur désactive le mod au lieu de faire tomber le jeu :
     * l'entrypoint Fabric ne doit pas empêcher le démarrage du serveur.
     */
    private void start() {
        try {
            lib = RustLib.start(RustLib.DEFAULT_TICK_BUDGET_NANOS, nativeResource());
            LOGGER.info("RustLib démarré ({}), registres : {} octets de JSON",
                    nativeResource(), lib.registriesJson().length());
        } catch (RuntimeException error) {
            lib = null;
            LOGGER.error("RustLib indisponible, RustLib Mods est désactivé : {}",
                    error.getMessage());
        }
    }

    /** Un tick Rust par tick de serveur, dans le fil du serveur. */
    private void tick() {
        if (lib == null) {
            return;
        }
        long executed = lib.tick();
        if (ticks++ == 0) {
            LOGGER.info("Premier tick Rust exécuté ({} tâche(s) planifiée(s))", executed);
        }
    }

    /** Libère le runtime Rust quand le serveur s'arrête (idempotent). */
    private void stop() {
        if (lib == null) {
            return;
        }
        lib.close();
        lib = null;
        LOGGER.info("RustLib arrêté");
    }

    /** Nom du natif compilé depuis {@code examples/rustlib-mods} pour cette plateforme. */
    private static String nativeResource() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        if (os.contains("win")) {
            return "/native/rustlib_mods.dll";
        }
        if (os.contains("mac")) {
            return "/native/librustlib_mods.dylib";
        }
        return "/native/librustlib_mods.so";
    }
}
