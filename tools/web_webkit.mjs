// B453: the browser build in WebKit (the system's WebKitGTK MiniBrowser: Safari's engine — JavaScriptCore, its wasm
// tiers, OPFS — on Linux) with what each frame cost, for hangs that only Safari shows. Serves web/ itself and adds a
// script to index.html that sends the page's console and every main-thread block back here: `raf gap N ms` (two
// animation frames that far apart) and `blocked N ms` (a 50 ms timer that late: the thread was busy, frame or not).
// WebKitGTK's OPFS has no sync access handles (the page's import), so a fresh profile gets the native build's disc pack
// (~/.local/share/hst/disc.pack, same format) written into OPFS first. Needs WAYLAND_DISPLAY or DISPLAY (MiniBrowser
// opens a window). Usage: node tools/web_webkit.mjs PACK OUT [query] [seconds]
// KEYS="20:Enter,26:ArrowDown" presses keys at those seconds after the game's first frame. OUT/console.txt: the log,
// each line with ms since the page loaded. PROFILE=dir keeps WebKit's data (OPFS: the imported disc) elsewhere than
// OUT/profile, so later runs skip writing the pack. WEB_PORT picks the port (8766). WK_FEATURES: MiniBrowser's --features
// (default: OPFS on, which Safari has and WebKitGTK ships off; add ',+WebGPU' for its WebGPU).
// B689: GUEST=1 opens the page as an adhoc guest (adhoc.rs): this server answers the host's routes itself
// (`/adhoc/info.json` with `lobby: true`, `/adhoc/disc` from PACK with byte ranges, `/adhoc/mods/…` from MODS, a mods
// folder: every mod in it, its texture packs too, as a host with them switched on serves them), and no OPFS setup.
// Every run samples the browser's processes' memory (RSS, all of WebKit's children) once a second: OUT/memory.txt has
// the peak of each and of their sum, the sum's timeline beside the console's ms. FLAKY=N (with GUEST): the first N
// info.json asks answer 503 and the first disc download is cut off after 20 MB, for the page's retries. What the page
// sends the host (`POST /adhoc/log`) goes to OUT/guest-log.txt. WebKitGTK has no WebRTC: a guest page joins a real
// host over its WebSocket then (M156; this server has none, so the lobby isn't joined here).
import { spawn } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, resolve } from 'node:path';

const [pack, out, query = 'fps&webgl2', secs = '90'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const web = resolve(new URL('../web', import.meta.url).pathname);
const port = +(process.env.WEB_PORT ?? 8766);
const keys = (process.env.KEYS ?? '').split(',').filter(Boolean).map(k => k.split(':')).map(([s, key]) => [+s, key]);
const log = [];
const guest = !!process.env.GUEST, modsDir = process.env.MODS ? resolve(process.env.MODS) : '';
const started = Date.now();
const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.webmanifest': 'application/manifest+json' };

// runs before the page's own scripts
const probe = `<script>
(() => {
  const t0 = performance.now(), buf = [];
  const send = () => { if (buf.length) fetch('/__log', { method: 'POST', body: buf.splice(0).join('\\n'), keepalive: true }).catch(() => {}); };
  const add = (type, args) => buf.push(\`\${Math.round(performance.now() - t0)}ms \${type}: \${[...args].map(a => typeof a === 'string' ? a.replace(/%c/g, '') : a instanceof Error ? a.stack || a.message : (() => { try { return JSON.stringify(a); } catch { return String(a); } })()).filter(a => !/^(color|font-style):/.test(a)).join(' ')}\`);
  for (const k of ['log', 'info', 'warn', 'error', 'debug']) { const f = console[k].bind(console); console[k] = (...a) => { add(k, a); f(...a); }; }
  addEventListener('error', e => add('exception', [e.message + ' ' + (e.filename ?? '') + ':' + (e.lineno ?? '')]));
  addEventListener('unhandledrejection', e => add('rejection', [String(e.reason) + ' ' + (e.reason?.stack ?? '')]));
  setInterval(send, 500);
  addEventListener('pagehide', send);
  try { localStorage.setItem('hst-autoplay', '1'); } catch {}
  let last = performance.now();
  const raf = t => { if (t - last > 100) add('probe', [\`raf gap \${Math.round(t - last)} ms\`]); last = t; requestAnimationFrame(raf); };
  requestAnimationFrame(raf);
  let beat = performance.now();
  setInterval(() => { const n = performance.now(); if (n - beat > 250) add('probe', [\`blocked \${Math.round(n - beat - 50)} ms\`]); beat = n; }, 50);
  // the boot and import status lines as they change
  let said;
  setInterval(() => { const s = [document.getElementById('status'), document.getElementById('step')].map(e => e?.textContent ?? '').join(' | '); if (s !== said) add('status', [said = s]); }, 1000);
  // keys from the game's first frame on
  const keys = ${JSON.stringify(keys)};
  const wait = setInterval(() => {
    const c = document.querySelector('canvas');
    if (!c) return;
    clearInterval(wait);
    add('probe', ['canvas up, keys start']);
    for (const [s, key] of keys) setTimeout(() => {
      add('probe', [\`key \${key}\`]);
      const code = key.length === 1 ? 'Key' + key.toUpperCase() : key;
      c.focus();
      c.dispatchEvent(new KeyboardEvent('keydown', { key, code, bubbles: true }));
      setTimeout(() => c.dispatchEvent(new KeyboardEvent('keyup', { key, code, bubbles: true })), 80);
    }, s * 1000);
  }, 200);
})();
</script>`;

const server = createServer((q, r) => {
  const u = new URL(q.url, 'http://x');
  if (u.pathname === '/__log') {
    let body = '';
    q.on('data', c => (body += c)).on('end', () => { log.push(...body.split('\n')); r.end(); });
    return;
  }
  if (guest && u.pathname.startsWith('/adhoc/')) return host(q, r, decodeURIComponent(u.pathname));
  if (u.pathname === '/__pack') return createReadStream(pack).pipe(r.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': statSync(pack).size }));
  // OPFS disc.pack from the native one unless it's there already, then the game
  if (u.pathname === '/__setup') return r.writeHead(200, { 'Content-Type': 'text/html' }).end(`<script type="module">
    const log = t => fetch('/__log', { method: 'POST', body: 'setup: ' + t });
    try {
      const dir = await navigator.storage.getDirectory();
      const have = await dir.getFileHandle('disc.pack').then(h => h.getFile()).then(f => f.size, () => 0);
      if (have !== ${statSync(pack).size}) {
        const w = await (await dir.getFileHandle('disc.pack', { create: true })).createWritable();
        await (await fetch('/__pack')).body.pipeTo(w);
        await log('pack written');
      }
      location = '/?${query}';
    } catch (e) { await log(String(e)); }
  </script>`);
  const path = join(web, u.pathname === '/' ? 'index.html' : decodeURIComponent(u.pathname));
  if (!path.startsWith(web)) return r.writeHead(403).end();
  try {
    let body = readFileSync(path);
    if (path.endsWith('index.html')) body = body.toString().replace('<head>', `<head>${probe}`);
    r.writeHead(200, { 'Content-Type': types[extname(path)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' }).end(body);
  } catch {
    r.writeHead(404).end();
  }
}).listen(port, '127.0.0.1');

// the host's mods (adhoc.rs `mods`/`files`): each folder under MODS with a mod.json, its files with their sizes
function hostMods() {
  if (!modsDir) return [];
  const walk = (base, d, out = []) => {
    for (const e of readdirSync(d, { withFileTypes: true }).sort((a, b) => a.name < b.name ? -1 : 1)) {
      if (e.name.startsWith('.')) continue;
      const p = join(d, e.name), st = statSync(p);
      st.isDirectory() ? walk(base, p, out) : out.push({ path: p.slice(base.length + 1), size: st.size });
    }
    return out;
  };
  const dirs = [];
  const look = (d, rel) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (!statSync(p).isDirectory()) continue;
      if (existsSync(join(p, 'mod.json'))) dirs.push([p, rel + e.name]); else if (!rel) look(p, e.name + '/');
    }
  };
  look(modsDir, '');
  return dirs.map(([p, folder]) => {
    let m = {};
    try { m = JSON.parse(readFileSync(join(p, 'mod.json'), 'utf8')); } catch {}
    const kind = m.kind === 'textures' || m.kind === 'court' || m.kind === 'umpire' ? m.kind : 'character';
    return { id: m.id ?? folder, name: m.name ?? folder, kind, folder, url: `/adhoc/mods/${folder.split('/').map(encodeURIComponent).join('/')}/`, files: walk(p, p), dir: p };
  });
}
const served = guest ? hostMods() : [];
const guestLog = [];
let flaky = +(process.env.FLAKY ?? 0), cut = flaky > 0;
function host(q, r, path) {
  const file = (f, type) => {
    const len = statSync(f).size, m = /^bytes=(\d+)-(\d*)$/.exec(q.headers.range ?? '');
    if (!m) return createReadStream(f).pipe(r.writeHead(200, { 'Content-Type': type, 'Content-Length': len, 'Accept-Ranges': 'bytes' }));
    const from = +m[1], to = m[2] ? Math.min(+m[2], len - 1) : len - 1;
    createReadStream(f, { start: from, end: to }).pipe(r.writeHead(206, { 'Content-Type': type, 'Content-Length': to - from + 1, 'Content-Range': `bytes ${from}-${to}/${len}` }));
  };
  if (path === '/adhoc/log') {
    let body = '';
    return q.on('data', c => (body += c)).on('end', () => { guestLog.push(...body.split('\n')); r.writeHead(204).end(); });
  }
  if (path === '/adhoc/info.json' && flaky > 0) return flaky--, r.writeHead(503).end();
  if (path === '/adhoc/disc' && cut && !q.headers.range) {
    cut = false;
    r.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': statSync(pack).size, 'Accept-Ranges': 'bytes' });
    return createReadStream(pack, { end: (20 << 20) - 1 }).on('end', () => r.destroy()).pipe(r, { end: false });
  }
  if (path === '/adhoc/info.json') {
    const mods = served.map(({ dir, ...m }) => m);
    return r.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }).end(JSON.stringify({ game: 'opentennis', version: 'dev', web: 'ready', disc: { url: '/adhoc/disc', name: 'disc.pack', size: statSync(pack).size, ready: true }, lobby: true, mods }));
  }
  if (path === '/adhoc/disc') return file(pack, 'application/octet-stream');
  for (const m of served) {
    const rest = path.slice('/adhoc/mods/'.length);
    if (!rest.startsWith(m.folder + '/')) continue;
    const f = resolve(m.dir, rest.slice(m.folder.length + 1));
    if (f.startsWith(m.dir + '/') && existsSync(f)) return file(f, 'application/octet-stream');
  }
  r.writeHead(404).end();
}

const data = resolve(process.env.PROFILE ?? join(out, 'profile'));
const browser = spawn('/usr/lib/webkit2gtk-4.1/MiniBrowser', ['--autoplay-policy=allow', `--features=${process.env.WK_FEATURES ?? '+StorageAPI,+StorageAPIEstimate,+FileSystem,+FileSystemWritableStream,+AccessHandle'}`, guest ? `http://localhost:${port}/?${query}` : `http://localhost:${port}/__setup`], {
  stdio: 'ignore',
  env: { ...process.env, XDG_DATA_HOME: join(data, 'data'), XDG_CACHE_HOME: join(data, 'cache'), XDG_CONFIG_HOME: join(data, 'config') },
});
// the browser's processes (MiniBrowser and every child: WebKitWebProcess holds the page, its wasm and JS) by RSS
const peaks = new Map(), timeline = [];
let sumPeak = 0;
const sample = () => {
  const kids = pid => { try { return readdirSync(`/proc/${pid}/task`).flatMap(t => readFileSync(`/proc/${pid}/task/${t}/children`, 'utf8').trim().split(/\s+/).filter(Boolean)); } catch { return []; } };
  const all = [], todo = [String(browser.pid)];
  while (todo.length) { const p = todo.pop(); all.push(p); todo.push(...kids(p)); }
  let sum = 0;
  for (const p of all) {
    try {
      const st = readFileSync(`/proc/${p}/status`, 'utf8');
      const name = /^Name:\s*(.+)$/m.exec(st)?.[1] ?? p, kb = +(/^VmRSS:\s*(\d+)/m.exec(st)?.[1] ?? 0);
      sum += kb;
      peaks.set(`${name} ${p}`, Math.max(peaks.get(`${name} ${p}`) ?? 0, kb));
    } catch {}
  }
  sumPeak = Math.max(sumPeak, sum);
  timeline.push(`${Date.now() - started}ms ${Math.round(sum / 1024)} MB`);
};
const sampler = setInterval(sample, 1000);
const stop = () => {
  clearInterval(sampler);
  const mb = kb => `${Math.round(kb / 1024)} MB`;
  writeFileSync(join(out, 'memory.txt'), [`peak (sum of the browser's processes): ${mb(sumPeak)}`, ...[...peaks].sort((a, b) => b[1] - a[1]).map(([n, kb]) => `  ${n}: ${mb(kb)}`), '', ...timeline].join('\n') + '\n');
  if (guest) writeFileSync(join(out, 'guest-log.txt'), guestLog.join('\n') + '\n');
  browser.kill(); server.close(); writeFileSync(join(out, 'console.txt'), log.join('\n') + '\n'); process.exit(0);
};
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, stop);
setTimeout(stop, +secs * 1000);
