#!/usr/bin/env python3
"""Two browsers play a 1v1 online, introduced by pasted codes.

After Raiders of the Lost Mart's tests/browser/online.py, cut to two seats.
`?ice=none` drops the STUN servers, so the browsers connect on host
candidates alone and no request leaves the origin; the room-code path
(Trystero over public relays) is checked by hand (HANDOFF.md).

The host makes an invitation, the joiner a reply; both reach the ready
state with the other's crew showing; the host starts two fights. It fails
on: either browser never getting ready, a crew not crossing, the two
browsers not seeing the same fight from opposite seats (each one's crew is
the other's foes, and one wins where the other loses), a desync reported,
a page or console error, or a request off the origin.

    python3 tests/browser/online.py [chromium firefox ...]   (default: chromium)
"""
import functools
import http.server
import json
import pathlib
import sys
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
failures = []


def check(ok, what):
    print(("  PASS " if ok else "  FAIL ") + what)
    if not ok:
        failures.append(what)


class Quiet(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def log_message(self, *a):
        pass


def serve():
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Quiet, directory=str(DIST)))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def names(page, side):
    return page.locator(f"#side-{side} .f-name b").all_inner_texts()


def run(p, engine, origin):
    print(f"\n[{engine}] online, two browsers")
    options = {
        "chromium": {"args": ["--disable-features=WebRtcHideLocalIpsWithMdns"]},
        "firefox": {"firefox_user_prefs": {"media.peerconnection.ice.obfuscate_host_addresses": False}},
    }.get(engine, {})
    browser = getattr(p, engine).launch(**options)
    problems, offsite = [], []
    pages = []
    for who in ("host", "joiner"):
        ctx = browser.new_context()
        page = ctx.new_page()
        page.on("console", lambda m, w=who: problems.append(f"{w}: {m.text}") if m.type == "error" else None)
        page.on("pageerror", lambda e, w=who: problems.append(f"{w}: {e}"))
        page.on("request", lambda r: None if r.url.startswith((origin, "blob:", "data:")) else offsite.append(r.url))
        page.goto(origin + "/index.html?ice=none")
        page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
        if who == "joiner":
            # a save at account level 3 with a different crew, so a crew
            # that failed to cross would show
            save = {"version": 1, "level": 3, "xp": 0, "units": [], "stars": [], "team": ["nan", "ed", "pip"]}
            page.evaluate("(s) => localStorage.setItem('level-twenty-save-v1', s)", json.dumps(save))
            page.reload()
            page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
        page.click("#b-online")
        page.wait_for_selector("body[data-mode='online']", state="attached", timeout=5000)
        page.click("#o-paste summary")
        pages.append(page)
    host, joiner = pages

    host.click("#o-new-invite")
    host.wait_for_selector("body[data-invite='ready']", state="attached", timeout=30000)
    invite = host.input_value("#o-invite-out")
    check(invite.startswith("O"), "the host makes an invitation")
    joiner.fill("#o-invite-in", invite)
    joiner.click("#o-make-reply")
    joiner.wait_for_selector("body[data-reply='ready']", state="attached", timeout=30000)
    reply = joiner.input_value("#o-reply-out")
    check(reply.startswith("A"), "the joiner makes a reply")
    host.fill("#o-reply-in", reply)
    host.click("#o-accept")
    try:
        host.wait_for_selector("body[data-online='online_host_ready']", state="attached", timeout=20000)
        joiner.wait_for_selector("body[data-online='online_joiner_ready']", state="attached", timeout=20000)
        ready = True
    except Exception:
        ready = False
    check(ready, "both browsers are connected and ready")
    check(host.locator("#o-foe .mini").count() == 3 and joiner.locator("#o-foe .mini").count() == 3, "each sees the other's crew of three")
    check(not host.is_disabled("#o-fight"), "the host may start")
    check(not joiner.is_visible("#o-fight"), "the joiner waits for the host")

    for round_ in (1, 2):
        host.click("#o-fight")
        for page in pages:
            page.wait_for_selector("body[data-mode='battle']", state="attached", timeout=15000)
        hn = (names(host, 0), names(host, 1))
        jn = (names(joiner, 0), names(joiner, 1))
        check(hn[0] == jn[1] and hn[1] == jn[0] and len(hn[0]) == 3, f"fight {round_}: each browser's crew is the other's foes {hn} {jn}")
        for page in pages:
            page.click("#a-skip")
            page.wait_for_selector("body[data-mode='result']", state="attached", timeout=15000)
        heads = [page.inner_text("#x-head") for page in pages]
        opposite = (heads == [COPY["won"], COPY["lost"]] or heads == [COPY["lost"], COPY["won"]] or heads == [COPY["drawn"], COPY["drawn"]])
        check(opposite, f"fight {round_}: one wins where the other loses {heads}")
        for page in pages:
            page.click("#x-back")
            page.wait_for_selector("body[data-mode='online']", state="attached", timeout=5000)
    statuses = [page.get_attribute("body", "data-online") for page in pages]
    check("online_desync" not in statuses, f"no desync reported {statuses}")
    check(COPY["online_fights"].replace("{n}", "2") in host.inner_text("#o-fights"), "the host counts two fights")

    joiner.click("#o-leave")
    host.wait_for_selector("body[data-online='online_left']", state="attached", timeout=15000)
    check(True, "the host is told when the joiner leaves")
    check(not problems, f"no page or console error {problems[:3]}")
    check(not offsite, f"no request off the origin {offsite[:3]}")
    browser.close()


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/ is missing: run scripts/build.sh first")
    engines = sys.argv[1:] or ["chromium"]
    srv = serve()
    origin = "http://127.0.0.1:%d" % srv.server_address[1]
    with sync_playwright() as p:
        for engine in engines:
            run(p, engine, origin)
    srv.shutdown()
    if failures:
        print(f"\nONLINE CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE CHECK PASSED in {', '.join(engines)}")


if __name__ == "__main__":
    main()
