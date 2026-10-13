// The transport: browsers, one reliable channel each, and no server of ours.
//
// Vagrancy's web/rtc.js, which was ported from Floodline's web/quad_rtc.js
// (its `window.FLOODLINE_RTC` API, :683-710) with the miniquad plugin glue
// removed. Ribbit King used it, and Raiders of the Lost Mart and Level Twenty use it unchanged. Its host side already
// takes any number of joiners (a peer per browser), so two seats need no
// new code here; the changes are the channel's label, the error keys in
// copy.json's words, and a pasted-code link closing being a leaving whatever
// the host's mode (a host with a room code may also invite by pasted code).
//
// This file and web/net/ are the one place in the page that speaks WebRTC.
// They sit outside the page scan (crates/core/tests/boundary.rs:
// `the_transport_stays_in_web_net`) because a transport needs timers, byte
// values and RTCPeerConnection, and they decide no rule of the game.
// Two introductions reach the same object, one RTCPeerConnection with one
// negotiated, ordered data channel:
//
//   * a room code: Trystero, over public Nostr relays this project does not
//     run, introduces the two browsers; the room name carries the build hash
//     so two builds cannot meet;
//   * a pasted code: the host's invitation and the joiner's reply carry the
//     whole handshake, compressed, and no relay is involved.
//
// The first byte on the channel says host or joiner, which keeps the star:
// a room is a mesh, and two joiners may meet and must ignore each other.
// Everything above this file sees only `onPeer`, `onLeft`, `onMessage`,
// `onError` and `send`.

import { CONFIG } from "./config.js";

const CHANNEL_ID = 40;
// Every message carries one byte saying what it is. A role says who the
// sender is; an ack says the sender has heard the other's role; data is the
// game's.
const ROLE_HOST = 0x48; // 'H'
const ROLE_JOINER = 0x4a; // 'J'
const ACK = 0x41; // 'A'
const DATA = 0x44; // 'D'
// A message that reaches a negotiated channel before the other side has
// created its end of it is lost, and the two sides create theirs whenever
// Trystero tells each of them about the other, which is not at the same
// moment. The first version sent its role once and read the first message
// as the role, so between two computers the host read the joiner's Hello as
// a role, refused it and hung up (reproduced locally: a joiner's first
// message from the host was "8,0", its role byte lost). So the role is
// repeated until the other side acknowledges it, and data waits until the
// other side's role has arrived, which proves its end exists.
const ROLE_REPEAT_MS = 250;
const GATHER_TIMEOUT_MS = 6000;

// `?ice=none` drops the STUN servers, so two tabs on one machine connect on
// host candidates alone and no request leaves the origin. tests/browser/online.py
// uses it; a player never needs it.
function rtcConfig() {
  const ice = new URLSearchParams(location.search).get('ice');
  return ice === 'none' ? { iceServers: [] } : CONFIG.rtcConfig;
}

export function open({ isHost, mode, room, build, onPeer, onLeft, onMessage, onError }) {
  const s = { isHost, closed: false, trystero: null, pending: null, hostPeer: null, nextId: 1, nextCode: 1 };

  // A peer is the other browser, whatever connections come and go under it.
  // Trystero may replace a connection to the same browser (a renegotiation,
  // or a retry after a failed attempt); the first version treated the new
  // connection as a stranger, turned it away as "full", and kept sending to
  // the dead one, so Start never arrived and the match froze (reported by
  // Sam between two computers). Now a peer keeps one id, its current link is
  // swapped in place, and what cannot be sent yet waits in its queue.
  const peers = new Map(); // key -> { id, key, link, queue, reported }
  const byId = new Map();
  function peerFor(key) {
    if (!peers.has(key)) {
      const peer = { id: s.nextId++, key, link: null, queue: [], reported: false };
      peers.set(key, peer);
      byId.set(peer.id, peer);
    }
    return peers.get(key);
  }

  function open(link) {
    return link && !link.gone && link.ready && link.ch.readyState === 'open';
  }

  function flush(peer) {
    while (peer.queue.length && open(peer.link)) {
      const body = peer.queue[0];
      const framed = new Uint8Array(body.length + 1);
      framed[0] = DATA;
      framed.set(body, 1);
      try { peer.link.ch.send(framed); } catch (e) { return; }
      peer.queue.shift();
    }
  }

  function attach(pc, mine, key) {
    const peer = peerFor(key);
    const link = { pc, mine, ch: null, role: 0, ready: false, acked: false, gone: false, peer, timer: null, early: [] };
    const ch = pc.createDataChannel('twenty', { negotiated: true, id: CHANNEL_ID, ordered: true });
    ch.binaryType = 'arraybuffer';
    link.ch = ch;
    const sayRole = () => {
      if (link.gone || link.acked || ch.readyState !== 'open') return;
      try { ch.send(new Uint8Array([isHost ? ROLE_HOST : ROLE_JOINER])); } catch (e) { /* closing */ }
    };
    ch.onopen = () => {
      sayRole();
      link.timer = setInterval(sayRole, ROLE_REPEAT_MS);
    };
    ch.onmessage = (e) => {
      const bytes = new Uint8Array(e.data);
      const kind = bytes[0];
      if (kind === ACK) {
        link.acked = true;
        clearInterval(link.timer);
        return;
      }
      if (kind === ROLE_HOST || kind === ROLE_JOINER) {
        try { ch.send(new Uint8Array([ACK])); } catch (err) { /* closing */ }
        if (link.ready) return; // a repeat
        link.role = kind;
        // The star: a host talks to joiners; a joiner to one host.
        const ok = isHost ? kind === ROLE_JOINER : (kind === ROLE_HOST && (!s.hostPeer || s.hostPeer === peer));
        if (!ok) { hangUp(link); return; }
        if (!isHost) s.hostPeer = peer;
        link.ready = true;
        if (peer.link && peer.link !== link) hangUp(peer.link);
        peer.link = link;
        flush(peer);
        if (!peer.reported) { peer.reported = true; onPeer(peer.id); }
        // Data that beat this side's copy of the role here is delivered now.
        for (const b of link.early.splice(0)) onMessage(peer.id, b);
        return;
      }
      if (kind === DATA) {
        // The sender has heard our role, so its end exists; its own role may
        // still be a repeat away. Keep what it sent until then.
        if (link.ready) onMessage(peer.id, bytes.subarray(1));
        else link.early.push(bytes.slice(1));
      }
    };
    const lost = () => {
      if (link.gone) return;
      link.gone = true;
      clearInterval(link.timer);
      if (peer.link === link) peer.link = null;
      // By pasted code there is no relay to say a peer left; a closed link
      // is the leaving. By room code, Trystero says so (onPeerLeave), and a
      // closed link may be a connection being replaced.
      if (mode === 'code' || link.mine) leave(peer);
    };
    ch.onclose = lost;
    pc.addEventListener('connectionstatechange', () => {
      if (pc.connectionState === 'failed' || pc.connectionState === 'closed') lost();
    });
    return link;
  }

  function leave(peer) {
    if (peer.reported && !peer.left) { peer.left = true; onLeft(peer.id); }
  }

  function hangUp(link) {
    link.gone = true;
    clearInterval(link.timer);
    try { link.ch.close(); if (link.mine) link.pc.close(); } catch (e) { /* already closing */ }
  }

  // ---- a room code, through Trystero ----------------------------------------

  async function startRoom() {
    let lib;
    try {
      lib = await import(new URL(CONFIG.bundle, document.baseURI).href);
    } catch (e) {
      console.warn('the signalling bundle did not load', e);
      onError('online_no_relay');
      return;
    }
    if (s.closed) return;
    // The room is named by the protocol, not the build: two builds a deploy
    // apart still meet, and the host's hello check turns a mismatched one
    // away with a sentence. (Named by build, a friend with a page cached
    // from an earlier deploy looked for another room and waited for ever.)
    const name = `${CONFIG.protocol}-${room}`;
    let r;
    try {
      r = lib.joinRoom({ appId: CONFIG.appId, password: CONFIG.password, rtcConfig: rtcConfig(), relayConfig: CONFIG.relayConfig }, name, {
        onJoinError: (d) => {
          // A failed attempt to reach a browser we are already playing with
          // is Trystero retrying under a working link, not the end of it.
          const known = d && d.peerId && peers.get(d.peerId);
          if (known && known.reported && !known.left) { console.warn('a connection attempt failed under a working link', d); return; }
          onError(d && d.peerId ? 'online_no_path' : 'online_no_relay');
        },
      });
    } catch (e) {
      console.warn(e);
      onError('online_no_relay');
      return;
    }
    s.trystero = r;
    // Trystero 0.25.4 takes these as properties, as Floodline assigns them.
    // A peer is keyed by Trystero's id for it, so a replaced connection to
    // the same browser is the same peer.
    r.onPeerJoin = (pid) => {
      const pc = r.getPeers()[pid];
      if (pc) attach(pc, false, pid);
    };
    r.onPeerLeave = (pid) => {
      const peer = peers.get(pid);
      if (peer) leave(peer);
    };
    setTimeout(() => {
      if (s.closed || [...peers.values()].some((p) => p.reported)) return;
      let open = 0;
      try {
        const sockets = lib.getRelaySockets();
        for (const k in sockets) if (sockets[k] && sockets[k].readyState === 1) open++;
      } catch (e) { /* the strategy may not expose them */ }
      if (open === 0) onError('online_no_relay');
    }, CONFIG.relayTimeoutMs);
  }

  // ---- a pasted code ----------------------------------------------------------

  function gathered(pc) {
    if (pc.iceGatheringState === 'complete') return Promise.resolve();
    return new Promise((resolve) => {
      let done = false;
      const finish = () => { if (!done) { done = true; resolve(); } };
      pc.addEventListener('icegatheringstatechange', () => { if (pc.iceGatheringState === 'complete') finish(); });
      setTimeout(finish, GATHER_TIMEOUT_MS);
    });
  }

  async function invitation() {
    const pc = new RTCPeerConnection(rtcConfig());
    // each invitation is its own browser: Vagrancy keyed every pasted-code
    // link 'code', which with two seats was one peer, and with four made a
    // second joiner's link replace (and hang up) the first's
    s.pending = attach(pc, true, `code-${s.nextCode++}`);
    await pc.setLocalDescription(await pc.createOffer());
    await gathered(pc);
    return pack('O', pc.localDescription.sdp);
  }

  async function reply(code) {
    const sdp = await unpack('O', code);
    const pc = new RTCPeerConnection(rtcConfig());
    s.pending = attach(pc, true, 'code');
    await pc.setRemoteDescription({ type: 'offer', sdp });
    await pc.setLocalDescription(await pc.createAnswer());
    await gathered(pc);
    return pack('A', pc.localDescription.sdp);
  }

  async function accept(code) {
    const sdp = await unpack('A', code);
    await s.pending.pc.setRemoteDescription({ type: 'answer', sdp });
  }

  if (mode === 'room') startRoom();

  return {
    // Host: the invitation to send. Joiner: given an invitation, the reply.
    invitation,
    reply,
    // Host: the joiner's reply.
    accept,
    // Never drops: what cannot go now waits for the peer's open link.
    send(id, bytes) {
      const peer = byId.get(id);
      if (!peer) return false;
      peer.queue.push(bytes);
      flush(peer);
      return true;
    },
    close() {
      s.closed = true;
      if (s.trystero) { try { s.trystero.leave(); } catch (e) { /* gone */ } }
      peers.forEach((p) => p.link && hangUp(p.link));
      peers.clear();
      byId.clear();
    },
  };
}

// ---- the blob: a data-channel SDP is nearly all boilerplate --------------------
//
// Only a handful of lines say anything the other end cannot infer, so the
// code carries those and `grow` rebuilds a valid description around them
// (Floodline quad_rtc.js `shrink`/`grow`).

function shrink(sdp) {
  const keep = [];
  for (const l of sdp.split(/\r\n|\n/)) {
    if (l.startsWith('a=ice-ufrag:')) keep.push('u' + l.slice(12));
    else if (l.startsWith('a=ice-pwd:')) keep.push('p' + l.slice(10));
    else if (l.startsWith('a=fingerprint:')) keep.push('f' + l.slice(14));
    else if (l.startsWith('a=setup:')) keep.push('s' + l.slice(8));
    else if (l.startsWith('a=mid:')) keep.push('m' + l.slice(6));
    else if (l.startsWith('a=sctp-port:')) keep.push('P' + l.slice(12));
    else if (l.startsWith('a=max-message-size:')) keep.push('M' + l.slice(19));
    else if (l.startsWith('a=candidate:') && / (udp|UDP) /.test(l)) keep.push('c' + l.slice(12));
  }
  return keep.join('\n');
}

function grow(min) {
  let ufrag = '', pwd = '', fp = '', setup = 'actpass', mid = '0', port = '5000', mms = '262144';
  const cands = [];
  for (const line of min.split('\n')) {
    const k = line[0];
    const v = line.slice(1);
    if (k === 'u') ufrag = v;
    else if (k === 'p') pwd = v;
    else if (k === 'f') fp = v;
    else if (k === 's') setup = v;
    else if (k === 'm') mid = v;
    else if (k === 'P') port = v;
    else if (k === 'M') mms = v;
    else if (k === 'c') cands.push('a=candidate:' + v);
  }
  return ['v=0', 'o=- 0 0 IN IP4 127.0.0.1', 's=-', 't=0 0', 'a=group:BUNDLE ' + mid, 'a=msid-semantic: WMS',
    'm=application 9 UDP/DTLS/SCTP webrtc-datachannel', 'c=IN IP4 0.0.0.0', ...cands,
    'a=ice-ufrag:' + ufrag, 'a=ice-pwd:' + pwd, 'a=ice-options:trickle', 'a=fingerprint:' + fp,
    'a=setup:' + setup, 'a=mid:' + mid, 'a=sctp-port:' + port, 'a=max-message-size:' + mms, ''].join('\r\n');
}

const b64url = (bytes) => btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
function unb64url(text) {
  const s = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(s, (c) => c.charCodeAt(0));
}
async function through(stream, bytes) {
  const w = stream.writable.getWriter();
  w.write(bytes);
  w.close();
  return new Uint8Array(await new Response(stream.readable).arrayBuffer());
}

// "O" for an invitation, "A" for a reply, outside the compressed part, so a
// code pasted into the wrong box is refused with a sentence.
async function pack(kind, sdp) {
  return kind + b64url(await through(new CompressionStream('deflate-raw'), new TextEncoder().encode(shrink(sdp))));
}

export class BadCode extends Error {}

async function unpack(kind, code) {
  const text = (code || '').replace(/\s+/g, '');
  if (text[0] !== kind) throw new BadCode('wrong kind');
  try {
    return grow(new TextDecoder().decode(await through(new DecompressionStream('deflate-raw'), unb64url(text.slice(1)))));
  } catch (e) {
    throw new BadCode(String(e));
  }
}
