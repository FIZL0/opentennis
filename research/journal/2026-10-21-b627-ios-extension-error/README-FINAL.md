# B627 — iOS Safari launch: "The game stopped: … contentScriptData.init_ts"

## Cause
`contentScriptData` / `init_ts` appear nowhere in our code (web/, crates/, pkg glue). It's an extension's content
script: Safari runs extension content scripts in the page, and their uncaught errors reach the page's `error` /
`unhandledrejection` listeners. `web/index.html`'s handlers passed anything that wasn't a bare "Script error." to
`fail()`, which shows the stop screen. (The B330 case only covered cross-origin errors the browser masks to
"Script error." with no file; WebKit gives extension errors a real message and a `webkit-masked-url://hidden/` file.)

## Fix (web/index.html, next to the existing handlers)
`foreign(filename, err)`: an error is ignored (console.warn) when
- its `filename` is set and isn't same-origin (webkit-masked-url://, safari-web-extension://, chrome-extension://,
  moz-extension://, another site), or
- it has no filename and every URL in its stack is from another origin (rejections have no filename).
Everything of ours (page inline scripts, modules, wasm-bindgen glue, wasm frames, blob: workers) is same-origin, so
it still stops the game as before. An error with neither a file nor stack URLs can't be attributed and still stops it.
`crates/hst-base/src/web.rs` needed no change: the handler lives in the page.

Checked: node run of `foreign` over Safari/Chrome stack formats (extension-only stack, mixed stack, wasm panic stack,
blob URL, no stack); `node --check` of the edited script block.

## Not verified / not 1:1
- Not reproduced on an iPhone (no iOS device in an unattended run): the fix rests on WebKit reporting extension
  errors with a masked/extension URL as filename or in the stack, which is how WebKit/Chromium/Gecko report them. If
  the user still sees it, Copy the error's report shows whether the error came with a file.
- Not 1:1 question: web front-end only, no original behaviour involved.
