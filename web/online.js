// Online play, as the page sees it: opens the transport (web/net/rtc.js),
// hands the core every peer, departure and message, and sends what the core
// leaves in its outbox. The core decides everything; this file only carries.
// Raiders of the Lost Mart's web/online.js, cut to two seats.
import * as rtc from "./net/rtc.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export function makeOnline(game, build) {
  let link = null;

  function open(isHost, mode, room) {
    close();
    link = rtc.open({
      isHost,
      mode,
      room,
      build,
      onPeer: (id) => {
        game.net_peer(id);
        pump();
      },
      onLeft: (id) => game.net_left(id),
      onMessage: (id, bytes) => {
        game.net_message(id, decoder.decode(bytes));
        pump();
      },
      onError: (key) => game.net_error(key),
    });
    return link;
  }

  function close() {
    if (link) link.close();
    link = null;
  }

  function pump() {
    if (!link) return;
    for (const [id, text] of JSON.parse(game.net_outbox())) link.send(id, encoder.encode(text));
  }

  return {
    // Host with a room code (Trystero over public relays) or by pasted codes.
    host(mode) {
      const room = game.host_online(build, Date.now());
      open(true, mode, room);
      return room;
    },
    // Join by a room code; false if it cannot be one.
    join(code) {
      if (!code.trim() || !game.join_online(build, code)) return false;
      open(false, "room", game.room());
      return true;
    },
    // Join by a pasted invitation: resolves to the reply to send back.
    joinByCode(invitation) {
      game.join_online(build, "");
      return open(false, "code", "").reply(invitation);
    },
    invitation: () => (link ? link.invitation() : Promise.reject(new Error("not hosting"))),
    accept: (reply) => (link ? link.accept(reply) : Promise.reject(new Error("not hosting"))),
    leave() {
      game.leave_online();
      close();
    },
    pump,
    active: () => Boolean(link),
    close,
  };
}
