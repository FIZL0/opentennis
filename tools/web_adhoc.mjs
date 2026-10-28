// M152b: a native host takes a browser guest over WebRTC with no codes passed by hand: the guest page opens with the
// host's signalling address (`?code=rtch<ip>:<port>`), POSTs its offer to the host's /rtc and connects with the reply.
// Usage: tools/web_adhoc.sh OUT [seconds of match] (starts the native host and web/ on :8766; this script is the browser
// half): node tools/web_adhoc.mjs ISO|- OUT HOST:PORT [seconds]. ISO `-`: OUT/profile already holds the pack.
// Every seat is a bot on both ends (`?autoplay`, the host's HST_AUTOPLAY). Console and screenshots into OUT.
// NORTC=1: the guest has no RTCPeerConnection (M156: it joins over the adhoc server's WebSocket; tools/web_guest.sh).
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const [iso, out, signal, secs = '60'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const web = process.env.WEB_PORT ?? 8766, cdp = process.env.CDP_PORT ?? 9336;
const base = `http://localhost:${web}/`;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const log = [];
const say = s => { log.push(s); console.log(s); };

const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${cdp}`, `--user-data-dir=${resolve(process.env.PROFILE ?? resolve(out, 'profile'))}`,
  '--ignore-gpu-blocklist', '--window-size=960,540', '--disable-background-timer-throttling', '--disable-renderer-backgrounding',
  '--disable-backgrounding-occluded-windows', ...(process.env.CHROME_FLAGS ?? '').split(' ').filter(Boolean), 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => { writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n'); chrome.kill(); });
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, () => process.exit(1));
let version;
for (let i = 0; i < 50 && !version; i++) {
  await sleep(200);
  version = await fetch(`http://127.0.0.1:${cdp}/json/version`).then(r => r.json()).catch(() => null);
}
const browser = new WebSocket(version.webSocketDebuggerUrl);
await new Promise(r => (browser.onopen = r));
const newWindow = () => new Promise(r => {
  browser.onmessage = ({ data }) => r(JSON.parse(data).result.targetId);
  browser.send(JSON.stringify({ id: 1, method: 'Target.createTarget', params: { url: 'about:blank', newWindow: true } }));
});

/// A page: its console lines, `send` (CDP), `keys`, `wait`, `stats`.
async function tab(name, url) {
  const target = await newWindow();
  const ws = new WebSocket(`ws://127.0.0.1:${cdp}/devtools/page/${target}`);
  await new Promise(r => (ws.onopen = r));
  let id = 0;
  const waiting = new Map();
  const lines = [];
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id && waiting.has(m.id)) waiting.get(m.id)(m), waiting.delete(m.id);
    let l;
    if (m.method === 'Runtime.consoleAPICalled') l = `${m.params.type}: ${m.params.args.map(a => a.value ?? a.description).join(' ')}`;
    if (m.method === 'Runtime.exceptionThrown') l = `exception: ${m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text}`;
    if (l) {
      l = l.replace(/%c/g, '').replace(/ color: [^%]*$/, '');
      lines.push(l);
      log.push(`[${name}] ${l}`);
    }
  };
  const send = (method, params = {}) => new Promise(r => { waiting.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
  await send('Runtime.enable');
  await send('Page.enable');
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `{
    const P = RTCPeerConnection; window.__pcs = [];
    window.RTCPeerConnection = function (...a) { const pc = new P(...a); __pcs.push(pc); return pc; };
    window.RTCPeerConnection.prototype = P.prototype;
    // M156: NORTC=1, a browser with WebRTC off (Lockdown Mode): the game joins over the host's WebSocket
    if (${!!process.env.NORTC}) delete window.RTCPeerConnection;
    // other agents' virtual pads would steer the menus
    Navigator.prototype.getGamepads = () => [];
    addEventListener('gamepadconnected', e => e.stopImmediatePropagation(), true);
    // past the site page (M96) straight into the game, as its autoplay box does
    try { localStorage.setItem('hst-autoplay', '1'); } catch {}
    window.__frames = 0; const f = () => { __frames++; requestAnimationFrame(f); }; requestAnimationFrame(f);
  }` });
  await send('Page.navigate', { url });
  const keys = async (...ks) => {
    for (const key of ks) {
      await send('Page.bringToFront');
      // `click@x@y`: a click there (CSS pixels)
      const at = key.match(/^click@(\d+)@(\d+)$/);
      if (at) {
        for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: +at[1], y: +at[2], button: 'left', clickCount: 1 });
        await sleep(700);
        continue;
      }
      for (const type of ['rawKeyDown', 'keyUp']) {
        await send('Input.dispatchKeyEvent', { type, key, code: key.length === 1 ? `Key${key.toUpperCase()}` : key, windowsVirtualKeyCode: { Enter: 13, ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Escape: 27 }[key] ?? key.toUpperCase().charCodeAt(0) });
        await sleep(120);
      }
      await sleep(700);
    }
  };
  const shot = async n => {
    const s = await send('Page.captureScreenshot', { format: 'png' });
    if (s.result) writeFileSync(resolve(out, `${name}-${n}.png`), Buffer.from(s.result.data, 'base64'));
  };
  const wait = async (re, s) => {
    for (let i = 0; i < s * 4; i++) {
      const hit = lines.map(l => l.match(re)).find(Boolean);
      if (hit) return hit;
      await sleep(250);
    }
    return null;
  };
  const stats = () => evaluate(`(async () => [__frames, ...await Promise.all(__pcs.map(async pc => pc.connectionState + ' ' +
    [...(await pc.getStats()).values()].filter(s => s.type == 'data-channel').map(s => s.label + ':' + s.messagesSent + '/' + s.messagesReceived).join(' ')))].join(' | '))()`);
  return { name, lines, send, evaluate, keys, shot, wait, stats };
}

if (iso !== '-') {
  const imp = await tab('import', `${base}?webgl2`);
  for (let i = 0; i < 100 && !(await imp.evaluate("!document.getElementById('disc')?.hidden")); i++) await sleep(200);
  let node;
  for (let i = 0; i < 50 && !node?.result?.nodeId; i++) {
    if (i) await sleep(200);
    const doc = await imp.send('DOM.getDocument');
    node = await imp.send('DOM.querySelector', { nodeId: doc.result.root.nodeId, selector: '#iso' });
  }
  await imp.send('DOM.setFileInputFiles', { nodeId: node.result.nodeId, files: [resolve(iso)] });
  for (let i = 0; i < 600 && !/Imported|Couldn't/.test(await imp.evaluate("document.getElementById('status').textContent")); i++) await sleep(1000);
  say(`import: ${await imp.evaluate("document.getElementById('status').textContent")}`);
  await imp.send('Page.navigate', { url: 'about:blank' });
}

// the guest opens the host's address as a code: Online → Join by Code → singles → controllers → character
// M152c: GUEST_URL, the page as the host serves it (it finds the lobby by itself)
const guest = await tab('guest', process.env.GUEST_URL ?? `${base}?code=rtch${signal}&webgl2&autoplay`);
say(`guest menu: ${!!(await guest.wait(/loading screen down/, 120))}`);
await sleep(3000);
await guest.keys('ArrowDown', 'Enter');
// in the host's lobby once the connection is up: then Ready
for (let i = 0; i < 60 && !/connected/.test(await guest.stats()); i++) await sleep(500);
say(`joined: ${await guest.stats()}`);
await sleep(4000);
await guest.shot('joining');
await guest.keys(...(process.env.WEB_GUEST ?? 'click@850@410').split(','));
await guest.shot('lobby');
const started = await guest.wait(/starting the online match/, 60);
say(`started: ${!!started}`);
for (let s = 2; s <= 10; s += 2) {
  await sleep(2000);
  say(`${s}s guest: ${await guest.stats()}`);
}
for (let s = 20; s <= +secs; s += 10) {
  await sleep(10000);
  await guest.shot(`match-${s}s`);
  say(`${s}s guest: ${await guest.stats()}`);
}
say(`guest: ${guest.lines.filter(l => /rollback tick|online tick|match over|desync|rollback:|panicked|exception|LAN/i.test(l)).join(' | ')}`);
chrome.kill();
process.exit(0);
