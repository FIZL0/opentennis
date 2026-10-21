// P64's check of the page's start-up mod listing (index.html `start`: every installed mod's files from OPFS, the
// manifests and select pictures read in, the rest only sized) in WebKit (the system's WebKitGTK MiniBrowser:
// Safari's engine and OPFS on Linux), the old one-file-at-a-time walk against the parallel one. A fresh profile gets a
// copy of a native mods folder written into OPFS first: the same tree, manifests and pictures whole, every other file
// a 1-byte stand-in (a listing only asks its size; FULL=1 copies those whole too, for playing them). Prints each run's milliseconds and whether both saw the same
// files. Needs WAYLAND_DISPLAY or DISPLAY. Usage: node research/journal/2026-10-20-p64-safari-start/scripts/web_opfs_mods.mjs [MODS] [OUT] [runs]
// (MODS defaults to ~/.local/share/hst/mods, OUT to context/p64; OUT/profile keeps the OPFS copy between runs: with
// WEB_PORT=8766 and PROFILE=OUT/profile, tools/web_webkit.mjs then starts the game with those mods installed.)
import { spawn } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync } from 'node:fs';
import { createServer } from 'node:http';
import { homedir } from 'node:os';
import { join, relative, resolve } from 'node:path';

const [mods = join(homedir(), '.local/share/hst/mods'), out = 'context/p64', runs = '3'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const port = +(process.env.WEB_PORT ?? 8767);
const small = n => n.endsWith('.json') || n.includes('/previews/');
const tree = [];
const list = d => { for (const e of readdirSync(d, { withFileTypes: true })) e.isDirectory() ? list(join(d, e.name)) : tree.push(relative(mods, join(d, e.name))); };
list(mods);
console.log(`${tree.length} files in ${readdirSync(mods).length} mods`);

// the page: the copy, then the two listings in turn (index.html's old loop and its current one, add_mod_* counted)
const page = `<script type="module">
const log = t => fetch('/__log', { method: 'POST', body: t });
const small = ${small};
try {
  const root = await navigator.storage.getDirectory();
  const tree = await (await fetch('/__tree')).json();
  if (!(await root.getFileHandle('copied').then(() => true, () => false))) {
    let i = 0;
    const put = async p => {
      let d = await root.getDirectoryHandle('mods', { create: true });
      const parts = ('' + p).split('/');
      for (const s of parts.slice(0, -1)) d = await d.getDirectoryHandle(s, { create: true });
      const w = await (await d.getFileHandle(parts.at(-1), { create: true })).createWritable();
      await w.write(${!!process.env.FULL} || small(p) ? await (await fetch('/__file?p=' + encodeURIComponent(p))).arrayBuffer() : new Uint8Array(1));
      await w.close();
      if (++i % 1000 === 0) await log('copied ' + i);
    };
    let k = 0;
    await Promise.all(Array.from({ length: 16 }, async () => { while (k < tree.length) await put(tree[k++]); }));
    await root.getFileHandle('copied', { create: true });
  }
  const mods = await root.getDirectoryHandle('mods');
  let seen;
  const add_mod_file = (p, b) => seen.push(p + ' ' + b.length), add_mod_lazy = (p, n) => seen.push(p + ' lazy');
  const old = async () => {
    const walk = async (d, path, out) => { for await (const [n, h] of d.entries()) h.kind === 'file' ? out.push([path + n, h]) : await walk(h, \`\${path}\${n}/\`, out); return out; };
    for await (const [id, h] of mods.entries()) {
      if (h.kind !== 'directory') continue;
      await h.getDirectoryHandle('textures').then(t => t.removeEntry('upscaled', { recursive: true })).catch(() => {});
      const files = await walk(h, \`mods/\${id}/\`, []);
      if (!files.some(f => f[0] === \`mods/\${id}/mod.json\`)) continue;
      let pack = false;
      try { pack = JSON.parse(await (await files.find(f => f[0] === \`mods/\${id}/mod.json\`)[1].getFile()).text()).kind === 'textures'; } catch (e) {}
      for (const [p, fh] of files) {
        const f = await fh.getFile();
        if (!pack && !p.endsWith('.json') && !p.includes('/previews/')) { add_mod_lazy(p, f.size); continue; }
        add_mod_file(p, new Uint8Array(await f.arrayBuffer()));
      }
    }
  };
  const cur = async () => {
    const say = () => {};
    let modBytes = 0;
    //START
    //END
  };
  for (let r = 0; r < ${+runs}; r++) {
    const res = [];
    for (const [name, f] of [['old', old], ['new', cur]]) {
      seen = [];
      const t = performance.now();
      await f();
      res.push([name, Math.round(performance.now() - t), seen.sort().join('\\n')]);
    }
    await log(\`run \${r}: \${res.map(([n, ms, s]) => \`\${n} \${ms} ms (\${s.split('\\n').length} files)\`).join(', ')}, same: \${res[0][2] === res[1][2]}\`);
  }
  await log('done');
} catch (e) { await log('error ' + e + ' ' + e.stack); }
</script>`;

// the current listing, cut out of index.html between its P64 markers
const html = readFileSync(new URL('../../../../web/index.html', import.meta.url), 'utf8');
const body = /\/\/ P64 listing\n([\s\S]*?)\n\s*\/\/ P64 listing end/.exec(html)?.[1];
if (!body) throw new Error('index.html has no P64 listing markers');

let done;
const finished = new Promise(r => (done = r));
const server = createServer((q, r) => {
  const u = new URL(q.url, 'http://x');
  if (u.pathname === '/__log') {
    let b = '';
    return q.on('data', c => (b += c)).on('end', () => { console.log(b); r.end(); if (b === 'done' || b.startsWith('error')) done(); });
  }
  if (u.pathname === '/__tree') return r.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(tree));
  if (u.pathname === '/__file') {
    const p = resolve(mods, u.searchParams.get('p'));
    if (!p.startsWith(resolve(mods))) return r.writeHead(403).end();
    return r.writeHead(200).end(readFileSync(p));
  }
  r.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' }).end(page.replace('//START\n    //END', body));
}).listen(port, '127.0.0.1');

const data = resolve(join(out, 'profile'));
const browser = spawn('/usr/lib/webkit2gtk-4.1/MiniBrowser', [`--features=${process.env.WK_FEATURES ?? '+StorageAPI,+StorageAPIEstimate,+FileSystem,+FileSystemWritableStream,+AccessHandle'}`, `http://localhost:${port}/__opfs`], {
  stdio: 'ignore',
  env: { ...process.env, XDG_DATA_HOME: join(data, 'data'), XDG_CACHE_HOME: join(data, 'cache'), XDG_CONFIG_HOME: join(data, 'config') },
});
const stop = () => { browser.kill(); server.close(); process.exit(0); };
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, stop);
setTimeout(() => { console.log('timed out'); stop(); }, +(process.env.SECS ?? 900) * 1000);
finished.then(stop);
