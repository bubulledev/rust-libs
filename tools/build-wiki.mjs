// Génère le site statique du wiki dans dist/wiki/ :
//   public/*.html  les pages rendues depuis wiki/*.md
//   public/style.css
//   worker.js      le Worker qui sert les fichiers statiques
//   manifest.json  la description attendue par Freebuff Sites
//
// Usage : npm run build:wiki
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { marked } from 'marked';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const WIKI_DIR = join(ROOT, 'wiki');
const OUT_DIR = join(ROOT, 'dist', 'wiki');
const PUBLIC_DIR = join(OUT_DIR, 'public');

const HOME = 'README.md';
const escapeHtml = (text) =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

// « 1. Introduction » → « 1-introduction »
const slug = (text) =>
  text
    .replace(/<[^>]+>/g, '')
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');

const hrefFor = (file) => (file === HOME ? 'index.html' : file.replace(/\.md$/, '.html'));
const displayTitle = (entry) => (entry.file === HOME ? 'Accueil' : entry.title);

// Sommaire : l'ordre et les titres viennent de wiki/SUMMARY.md.
const entries = [...readFileSync(join(WIKI_DIR, 'SUMMARY.md'), 'utf8').matchAll(/\[([^\]]+)\]\(([^)\s]+\.md)\)/g)]
  .map(([, title, file]) => ({ title: title.trim(), file }));

if (entries.length === 0 || entries[0].file !== HOME) {
  throw new Error('wiki/SUMMARY.md doit commencer par le lien vers README.md');
}

function renderMarkdown(markdown) {
  let html = marked.parse(markdown);

  // Les liens du wiki pointent des fichiers .md : le site sert des .html.
  html = html.replace(/href="([^"]+?)\.md((?:#[^"]*)?)"/g, (_, name, anchor) => {
    const target = name === 'README' ? 'index.html' : `${name}.html`;
    return `href="${target}${anchor}"`;
  });
  const localTargets = new Set(entries.map((entry) => hrefFor(entry.file)));
  for (const [, target] of html.matchAll(/href="([^"]+\.html(?:#[^"]*)?)"/g)) {
    const page = target.split('#')[0];
    if (!localTargets.has(page)) {
      throw new Error(`lien interne cassé dans le wiki : ${target}`);
    }
  }

  // Ancre stable sur chaque sous-titre, pour les liens profonds.
  const seen = new Map();
  return html.replace(/<h([2-4])>([\s\S]*?)<\/h\1>/g, (_, level, inner) => {
    let id = slug(inner);
    const count = seen.get(id) ?? 0;
    seen.set(id, count + 1);
    if (count > 0) id = `${id}-${count + 1}`;
    return `<h${level} id="${id}">${inner}</h${level}>`;
  });
}

const pageTitle = (markdown, fallback) => {
  const heading = markdown.match(/^#\s+(.+)$/m);
  return heading ? heading[1].trim() : fallback;
};

function layout({ title, body, current }) {
  const nav = entries
    .filter((entry) => entry.file !== HOME)
    .map(
      (entry) =>
        `        <li${entry.file === current ? ' class="active"' : ''}><a href="${hrefFor(entry.file)}">${escapeHtml(entry.title)}</a></li>`,
    )
    .join('\n');

  const index = entries.findIndex((entry) => entry.file === current);
  const previous = entries[index - 1];
  const next = entries[index + 1];
  const pager = [
    previous
      ? `        <a class="prev" href="${hrefFor(previous.file)}"><span class="dir">Précédent</span>${escapeHtml(displayTitle(previous))}</a>`
      : '',
    next
      ? `        <a class="next" href="${hrefFor(next.file)}"><span class="dir">Suivant</span>${escapeHtml(displayTitle(next))}</a>`
      : '',
  ]
    .filter(Boolean)
    .join('\n');

  const suffix = current === HOME ? '' : ` — ${escapeHtml(title)}`;

  return `<!doctype html>
<html lang="fr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Wiki RustLib${suffix}</title>
  <meta name="description" content="Wiki de RustLib : écrire la logique d'un mod Fabric en Rust.">
  <meta name="color-scheme" content="light dark">
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <header class="topbar">
    <a class="brand" href="index.html"><span class="brand-mark">R</span>RustLib <small>wiki</small></a>
    <span class="topbar-meta">MIT OR Apache-2.0</span>
  </header>
  <div class="layout">
    <nav class="sidebar" aria-label="Sommaire">
      <p class="nav-label">Sommaire</p>
      <ol class="nav-list">
${nav}
      </ol>
    </nav>
    <main>
      <article class="prose">
${body.trimEnd()}
      </article>
      <nav class="pager" aria-label="Navigation">
${pager}
      </nav>
      <footer class="site-footer">RustLib 0.1.0 — bibliothèque Rust pour mods Fabric.</footer>
    </main>
  </div>
</body>
</html>
`;
}

rmSync(OUT_DIR, { recursive: true, force: true });
mkdirSync(PUBLIC_DIR, { recursive: true });

for (const entry of entries) {
  const markdown = readFileSync(join(WIKI_DIR, entry.file), 'utf8');
  const html = layout({
    title: pageTitle(markdown, entry.title),
    body: renderMarkdown(markdown),
    current: entry.file,
  });
  writeFileSync(join(PUBLIC_DIR, hrefFor(entry.file)), html);
}

copyFileSync(join(ROOT, 'tools', 'wiki.css'), join(PUBLIC_DIR, 'style.css'));

writeFileSync(
  join(OUT_DIR, 'worker.js'),
  `export default {
  async fetch(request, env) {
    return env.ASSETS.fetch(request);
  },
};
`,
);

writeFileSync(
  join(OUT_DIR, 'manifest.json'),
  `${JSON.stringify(
    {
      entrypoint: 'worker.js',
      compatibilityDate: '2026-11-17',
      modules: [{ name: 'worker.js', type: 'esm', path: './worker.js' }],
      assetsDirectory: './public',
    },
    null,
    2,
  )}\n`,
);

console.log(`${entries.length} pages générées dans dist/wiki/public`);
