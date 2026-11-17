package dev.rustlib;

/**
 * Vérification de bout en bout : jar → bibliothèque native → JNI → runtime Rust.
 *
 * <p>Lancé par {@code build-jar.sh} après construction du jar. Aucun serveur
 * Minecraft n'est nécessaire : seule la classe {@link RustLib} est utilisée.
 */
public final class RustLibSmokeTest {

    /** Quota du mod d'exemple : casses de minerai autorisées par fenêtre. */
    private static final int ORE_QUOTA = 64;

    private RustLibSmokeTest() {
    }

    public static void main(String[] args) {
        long jsonLength;
        long executed = 0;
        int allowed = 0;

        try (RustLib lib = RustLib.start(RustLib.DEFAULT_TICK_BUDGET_NANOS)) {
            String json = lib.registriesJson();
            require(json != null && json.contains("copper:copper_ore"),
                    "JSON des registres incomplet : " + json);
            jsonLength = json.length();

            for (int i = 0; i < 5; i++) {
                long tick = lib.tick();
                require(tick >= 0, "tick invalide : " + tick);
                executed += tick;
            }

            for (int i = 0; i < ORE_QUOTA + 6; i++) {
                if (lib.blockBreak(0, 64, 0, "copper:copper_ore", "joueur")) {
                    allowed++;
                }
            }
            require(allowed == ORE_QUOTA,
                    "quota du mod : " + allowed + " casses autorisées, " + ORE_QUOTA + " attendues");

            require(lib.blockBreak(0, 64, 0, "minecraft:stone", "joueur"),
                    "la pierre doit rester cassable");
        }

        // Après close(), aucun handle ne doit repasser la frontière JNI.
        RustLib closed = RustLib.start();
        closed.close();
        try {
            closed.tick();
            throw new AssertionError("tick() après close() aurait dû échouer");
        } catch (IllegalStateException expected) {
            // comportement attendu
        }

        System.out.println("OK  jar + JNI + runtime Rust");
        System.out.println("    registres : " + jsonLength + " octets de JSON");
        System.out.println("    ticks     : 5 ticks, " + executed + " tache(s) executee(s)");
        System.out.println("    quota     : " + allowed + "/70 casses de minerai autorisees");
    }

    private static void require(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
