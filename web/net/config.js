// Deployment settings for online play, after Floodline's web/config.js by
// way of Vagrancy's. Edited here and copied as-is into the build.
export const CONFIG = {
  // The vendored Trystero bundle, pinned by sha256 in vendor/SHA256SUMS
  // (checked by scripts/build.sh) and loaded only when somebody hosts or
  // joins by room code.
  bundle: "net/vendor/trystero-nostr-0.25.4.js",
  // Keeps this game's rooms apart from every other Trystero app on the same
  // public relays. The room name adds the build hash and the room code, so
  // two builds of the game cannot meet.
  appId: "level-twenty-p2p",
  // Part of every room's name; bumped only when online play changes shape.
  protocol: "lt1",
  // Encrypts the session descriptions as they cross a relay this project
  // does not run. Not a secret: it ships in this file. Game traffic is
  // DTLS-encrypted end to end regardless.
  password: "level-twenty",
  // The relays a room code is introduced over. Trystero's own list (in the
  // bundle, `defaultRelayUrls`) includes relays that have since gone; in
  // October 2026 relay.mostr.pub redirected every handshake, which put a
  // console error on every page that hosted. These answered a WebSocket
  // upgrade then. Every player must use the same list, so it lives here and
  // not in a person's settings.
  relayConfig: {
    urls: [
      "wss://nos.lol",
      "wss://nostr.mom",
      "wss://relay.damus.io",
      "wss://relay.primal.net",
      "wss://nostr-01.yakihonne.com",
      "wss://nostr.sathoarder.com",
      "wss://relay.mostro.network",
    ],
  },
  // STUN tells each browser its public address. When both players are
  // behind strict NATs only a TURN relay can carry the traffic, and there is
  // no serverless substitute (copy.json's online_no_path says so).
  rtcConfig: {
    iceServers: [
      { urls: "stun:stun.l.google.com:19302" },
      { urls: "stun:global.stun.twilio.com:3478" },
    ],
  },
  // How long to wait for a relay to introduce anybody before saying so.
  relayTimeoutMs: 15000,
};
