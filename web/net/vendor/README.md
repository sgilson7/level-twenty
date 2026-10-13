# Vendored, pinned, and not fetched at build time

One Trystero strategy bundle: the same file, with the same hash, as Floodline's `web/vendor/` and Vagrancy's. `scripts/build.sh` checks it against `SHA256SUMS` before it builds, and `LICENSES.md` carries its row.

| file | version | sha256 |
|---|---|---|
| `trystero-nostr-0.25.4.js` | 0.25.4 | `6bfce15d72a64384cc66c2693917994e1b900f4f98c9b7a2d54e4e86f5202906` |

It is a self-contained ES module (no imports, nothing fetched at load time), produced by esm.sh's build service from the published npm package:

    curl -o trystero-nostr-0.25.4.js \
      https://esm.sh/@trystero-p2p/nostr@0.25.4/es2022/nostr.bundle.mjs

The page imports it only when somebody hosts or joins by room code (`web/net/rtc.js`), so the browser check, which never does, makes no request off the origin.
