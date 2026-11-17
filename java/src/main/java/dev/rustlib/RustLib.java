package dev.rustlib;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

/**
 * Point d'entrée Java de RustLib.
 *
 * <p>Cette classe est le seul code Java nécessaire pour piloter un mod écrit en
 * Rust : elle charge la bibliothèque native embarquée dans le jar, démarre le
 * mod, lui envoie un tick et transmet les événements annulables.
 *
 * <pre>{@code
 * try (RustLib lib = RustLib.start()) {   // 2 ms de budget par tick
 *     String json = lib.registriesJson(); // blocs et items à créer côté serveur
 *     lib.tick();                         // une fois par tick
 *     boolean ok = lib.blockBreak(x, y, z, "copper:copper_ore", "Alex");
 * }
 * }</pre>
 *
 * <p>Un seul chemin de chargement existe : le natif vient du jar. Tous les
 * appels doivent venir du fil du serveur, car le runtime Rust n'ajoute aucun
 * verrou (choix de performance).
 *
 * <p>Comme la classe ne dépend d'aucune classe Minecraft, elle compile et se
 * teste sans serveur (voir {@code java/src/test/java}).
 */
public final class RustLib implements AutoCloseable {

    /** Budget par défaut accordé aux tâches Rust : 2 ms par tick (tick = 50 ms). */
    public static final long DEFAULT_TICK_BUDGET_NANOS = 2_000_000L;

    /** Nom du fichier natif embarqué pour la plateforme courante. */
    private static final String LIBRARY_FILE_NAME = detectLibraryFileName();

    private static boolean libraryLoaded;

    /** Ressource classpath du natif effectivement chargé (une seule par JVM). */
    private static String loadedResource;

    // Méthodes natives, exportées par rustlib_jni::export_mod! côté Rust.
    private static native long nativeCreate(long budgetNanos);

    private static native String nativeRegistries(long handle);

    private static native long nativeTick(long handle);

    private static native boolean nativeBlockBreak(
            long handle, int x, int y, int z, String block, String player);

    private static native void nativeDestroy(long handle);

    private long handle;

    private RustLib(long handle) {
        this.handle = handle;
    }

    /** Démarre le mod avec le budget par défaut et le natif standard. */
    public static RustLib start() {
        return start(DEFAULT_TICK_BUDGET_NANOS, null);
    }

    /**
     * Démarre le mod embarqué dans la bibliothèque native.
     *
     * @param budgetNanos temps maximal accordé aux tâches Rust par tick
     * @throws IllegalStateException si le chargement du mod échoue côté Rust,
     *     avec le message d'erreur produit par Rust
     */
    public static RustLib start(long budgetNanos) {
        return start(budgetNanos, null);
    }

    /**
     * Démarre le mod avec un natif explicite.
     *
     * <p>Le nom de fichier du natif standard ({@code rustlib.dll},
     * {@code librustlib.so}…) est un détail de la bibliothèque ; un mod qui
     * compile sa propre crate Rust embarque plutôt son propre nom. Exemple :
     *
     * <pre>{@code
     * RustLib.start(RustLib.DEFAULT_TICK_BUDGET_NANOS, "/native/rustlib_mods.dll");
     * }</pre>
     *
     * @param nativeResource ressource du classpath, avec ou sans {@code /}
     *     initial, ou {@code null} pour le nom standard de la plateforme
     * @throws IllegalStateException si un autre natif a déjà été chargé dans
     *     cette JVM : les symboles JNI sont identiques, un seul natif par
     *     processus est donc autorisé
     */
    public static RustLib start(long budgetNanos, String nativeResource) {
        ensureLibraryLoaded(nativeResource);
        long handle = nativeCreate(budgetNanos);
        if (handle == 0L) {
            // Rust lève déjà une IllegalStateException détaillée ; ce garde-fou
            // couvre le cas d'un natif compilé sans la macro export_mod!.
            throw new IllegalStateException("RustLib : le mod n'a pas pu être chargé");
        }
        return new RustLib(handle);
    }

    /** JSON décrivant les blocs et items à créer côté Minecraft. */
    public String registriesJson() {
        return nativeRegistries(startedHandle());
    }

    /**
     * Exécute un tick Rust.
     *
     * @return nombre de tâches exécutées pendant ce tick
     */
    public long tick() {
        return nativeTick(startedHandle());
    }

    /**
     * Signale une casse de bloc.
     *
     * @param block identifiant {@code namespace:path} du bloc
     * @param player nom du joueur
     * @return {@code false} si un mod annule la casse (le bloc doit rester)
     */
    public boolean blockBreak(int x, int y, int z, String block, String player) {
        return nativeBlockBreak(startedHandle(), x, y, z, block, player);
    }

    /** Arrête le mod et libère l'état côté Rust. Idempotent. */
    @Override
    public void close() {
        if (handle != 0L) {
            try {
                nativeDestroy(handle);
            } finally {
                handle = 0L;
            }
        }
    }

    /** Garde-fou : un handle libéré ne doit jamais repasser la frontière JNI. */
    private long startedHandle() {
        if (handle == 0L) {
            throw new IllegalStateException("RustLib est arrêté : appelez RustLib.start()");
        }
        return handle;
    }

    /**
     * Extrait le natif du jar et le charge. Une seule fois, au premier usage.
     *
     * @param nativeResource ressource explicite, ou {@code null} pour le natif
     *     standard de la plateforme
     */
    private static synchronized void ensureLibraryLoaded(String nativeResource) {
        String resource = nativeResource == null
                ? "/native/" + LIBRARY_FILE_NAME
                : nativeResource.startsWith("/") ? nativeResource : "/" + nativeResource;
        if (libraryLoaded) {
            if (resource.equals(loadedResource)) {
                return;
            }
            throw new IllegalStateException(
                    "RustLib : " + loadedResource + " est déjà chargé dans cette JVM, " + resource
                            + " ne peut pas l'être aussi.");
        }
        String fileName = resource.substring(resource.lastIndexOf('/') + 1);
        try (InputStream input = RustLib.class.getResourceAsStream(resource)) {
            if (input == null) {
                throw new IllegalStateException(
                        "RustLib : " + resource + " absent du jar. Compilez la partie native "
                                + "pour cette plateforme (./build-jar.sh).");
            }
            Path directory = Files.createTempDirectory("rustlib-native");
            Path target = directory.resolve(fileName);
            Files.copy(input, target, StandardCopyOption.REPLACE_EXISTING);
            target.toFile().deleteOnExit();
            directory.toFile().deleteOnExit();
            System.load(target.toAbsolutePath().toString());
            libraryLoaded = true;
            loadedResource = resource;
        } catch (IOException error) {
            throw new UncheckedIOException(
                    "RustLib : extraction de la bibliothèque native impossible", error);
        }
    }

    private static String detectLibraryFileName() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        if (os.contains("win")) {
            return "rustlib.dll";
        }
        if (os.contains("mac")) {
            return "librustlib.dylib";
        }
        return "librustlib.so";
    }
}
