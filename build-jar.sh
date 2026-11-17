#!/usr/bin/env bash
# Construit dist/rustlib-<version>.jar : classes Java + bibliothèque native du
# mod d'exemple + fabric.mod.json + les deux licences sous META-INF/, vérifie la
# chaîne complète (chargement du jar, JNI, tick, événement annulé), puis publie
# la coordonnée dev.rustlib:rustlib:<version> dans dist/maven (dépôt Maven local
# consommé par fabric-mod/).
#
# Usage :
#   ./build-jar.sh
#   CARGO_FLAGS=--offline ./build-jar.sh
set -euo pipefail

cd "$(dirname "$0")"

MOD_CRATE="rustlib-mods"
MOD_LIB_NAME="rustlib_mods"
BUILD_DIR="java/build"
RESOURCES="java/src/main/resources"

# Version unique : celle déclarée pour Fabric.
VERSION="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    "${RESOURCES}/fabric.mod.json" | head -1)"
if [ -z "${VERSION}" ]; then
    echo "'version' introuvable dans ${RESOURCES}/fabric.mod.json" >&2
    exit 1
fi
JAR_PATH="dist/rustlib-${VERSION}.jar"

# Un outil du JDK : JAVA_HOME, PATH, puis emplacements d'installation courants
# (sous Windows, seul `java`/`javac` est souvent dans le PATH, pas `jar`).
find_tool() {
    local tool="$1"
    if [ -n "${JAVA_HOME:-}" ] && [ -x "${JAVA_HOME}/bin/${tool}" ]; then
        printf '%s\n' "${JAVA_HOME}/bin/${tool}"
        return 0
    fi
    if command -v "${tool}" >/dev/null 2>&1; then
        command -v "${tool}"
        return 0
    fi
    local candidate
    for candidate in "/c/Program Files/Java"/*/bin/"${tool}" \
                     "/c/Program Files/Java"/*/bin/"${tool}.exe" \
                     "/c/Program Files/Eclipse Adoptium"/*/bin/"${tool}.exe" \
                     /usr/lib/jvm/*/bin/"${tool}" \
                     /opt/java/*/bin/"${tool}"; do
        if [ -x "${candidate}" ]; then
            printf '%s\n' "${candidate}"
            return 0
        fi
    done
    return 1
}

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        NATIVE_SRC="target/release/${MOD_LIB_NAME}.dll"
        TARGET_LIB_NAME="rustlib.dll"
        CLASS_PATH_SEP=";"
        ;;
    Darwin)
        NATIVE_SRC="target/release/lib${MOD_LIB_NAME}.dylib"
        TARGET_LIB_NAME="librustlib.dylib"
        CLASS_PATH_SEP=":"
        ;;
    *)
        NATIVE_SRC="target/release/lib${MOD_LIB_NAME}.so"
        TARGET_LIB_NAME="librustlib.so"
        CLASS_PATH_SEP=":"
        ;;
esac

echo "== 1/6 compilation du Rust (release) =="
cargo build --release -p "${MOD_CRATE}" ${CARGO_FLAGS:-}
if [ ! -f "${NATIVE_SRC}" ]; then
    echo "bibliothèque native introuvable : ${NATIVE_SRC}" >&2
    exit 1
fi

echo "== 2/6 outils Java =="
if ! JAVAC_BIN="$(find_tool javac)"; then
    echo "javac introuvable : installez un JDK 21" >&2
    exit 1
fi
if ! JAVA_BIN="$(find_tool java)"; then
    echo "java introuvable : installez un JDK 21" >&2
    exit 1
fi
if ! JAR_BIN="$(find_tool jar)"; then
    echo "outil 'jar' introuvable : installez un JDK complet" >&2
    exit 1
fi

echo "== 3/6 compilation des classes Java =="
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/classes"
"${JAVAC_BIN}" -d "${BUILD_DIR}/classes" \
    $(find java/src/main/java -name '*.java')

echo "== 4/6 assemblage du jar =="
mkdir -p "${BUILD_DIR}/jar/native" dist
cp -R "${BUILD_DIR}/classes/." "${BUILD_DIR}/jar/"
cp -R "${RESOURCES}/." "${BUILD_DIR}/jar/"
# Le jar distribué transporte sa propre licence.
mkdir -p "${BUILD_DIR}/jar/META-INF"
cp LICENSE-MIT LICENSE-APACHE "${BUILD_DIR}/jar/META-INF/"
cp "${NATIVE_SRC}" "${BUILD_DIR}/jar/native/${TARGET_LIB_NAME}"
cat > "${BUILD_DIR}/MANIFEST.MF" <<EOF
Manifest-Version: 1.0
Implementation-Title: RustLib
Implementation-Version: ${VERSION}
EOF
rm -f "${JAR_PATH}"
"${JAR_BIN}" --create --file "${JAR_PATH}" \
    --manifest "${BUILD_DIR}/MANIFEST.MF" -C "${BUILD_DIR}/jar" .

echo "== 5/6 vérification (jar → JNI → Rust) =="
mkdir -p "${BUILD_DIR}/test-classes"
"${JAVAC_BIN}" -cp "${JAR_PATH}" -d "${BUILD_DIR}/test-classes" \
    $(find java/src/test/java -name '*.java')
"${JAVA_BIN}" -cp "${JAR_PATH}${CLASS_PATH_SEP}${BUILD_DIR}/test-classes" \
    dev.rustlib.RustLibSmokeTest

echo "== 6/6 publication dans le dépôt Maven local (dist/maven) =="
# Le mod Fabric déclare rustlib comme une dépendance Maven ordinaire :
#   dev.rustlib:rustlib:${VERSION}
# Ce dépôt local rend la coordonnée résoluble par Gradle, sans dépôt distant.
MAVEN_DIR="dist/maven/dev/rustlib/rustlib/${VERSION}"
mkdir -p "${MAVEN_DIR}"
cp "${JAR_PATH}" "${MAVEN_DIR}/rustlib-${VERSION}.jar"
cat > "${MAVEN_DIR}/rustlib-${VERSION}.pom" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>dev.rustlib</groupId>
  <artifactId>rustlib</artifactId>
  <version>${VERSION}</version>
  <packaging>jar</packaging>
  <name>RustLib</name>
  <description>Bibliothèque Rust pour écrire la logique de mods Minecraft Fabric</description>
  <licenses>
    <license><name>MIT</name></license>
    <license><name>Apache-2.0</name></license>
  </licenses>
</project>
EOF

echo
echo "Contenu de ${JAR_PATH} :"
"${JAR_BIN}" --list --file "${JAR_PATH}"
