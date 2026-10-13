"""The project's own interactions for tests/browser/check.py.

A new player's first minutes: the menu at account level 1, the crew screen
(nine of twelve locked; a locked unit refused; a swap kept), a campaign
fight played to the end and its results, a random fight skipped to the end,
the online screen opened without connecting, the help screen, and a reload
that keeps the progress. Selectors and waits only: Firefox's CSP blocks
Playwright's string-form wait_for_function.

Expected values are worked out from the brief and the data files by hand
(account level 1 at the start, three starters unlocked, twelve units), not
copied from the game's own output.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
COPY = json.loads((ROOT / "data" / "copy.json").read_text())


def images_loaded(page, scope):
    return page.evaluate(
        "(sel) => [...document.querySelectorAll(sel + ' img')].filter(i => i.getAttribute('src')).every(i => i.complete && i.naturalWidth > 0)",
        scope,
    )


def run(page, check, engine):
    page.wait_for_selector("body[data-mode='menu']", state="attached", timeout=10000)
    check(page.inner_text("#m-level") == "1", "a new player is account level 1")
    check(page.locator("#m-team .mini").count() == 3, "the menu shows a crew of three")
    check(images_loaded(page, "#menu"), "the TikZ title art and portraits loaded")

    # the crew screen
    page.click("#b-roster")
    page.wait_for_selector("body[data-mode='roster']", state="attached", timeout=5000)
    check(page.locator("#r-grid .unit-card").count() == 12, "the roster has twelve units")
    check(page.locator("#r-grid .unit-card.locked").count() == 9, "nine are locked at account level 1")
    check(images_loaded(page, "#roster"), "every portrait and kind icon loaded")
    page.click("#r-grid .unit-card[data-unit='chum']")
    check(page.inner_text("#r-error") == COPY["team_locked"], "a locked unit cannot join the crew")
    # put Pip in the first slot: Pip is already in the crew, so it is refused
    page.click("#r-slots .slot >> nth=0")
    page.click("#r-grid .unit-card[data-unit='vela']")
    slots = page.locator("#r-slots .mini-cap b").all_inner_texts()
    check(slots == ["Rook", "Vela", "Pip"], f"picking a unit already in the crew changes nothing {slots}")
    page.click("#roster button.back")

    # a campaign fight, watched to the end
    page.click("#b-campaign")
    page.wait_for_selector("body[data-mode='campaign']", state="attached", timeout=5000)
    check(page.locator("#c-chapter .stage").count() == 5, "the first part of town has five fights")
    check(page.locator("#c-chapter .stage button:disabled").count() == 4, "only the first fight is open")
    page.click("#c-chapter .stage button >> nth=0")
    page.wait_for_selector("body[data-mode='battle']", state="attached", timeout=5000)
    check(page.locator("#side-0 .fighter").count() == 3 and page.locator("#side-1 .fighter").count() == 3, "three against three")
    check(images_loaded(page, "#battle"), "the backdrop and fighters loaded")
    page.click("#a-speeds .speed >> nth=-1")
    page.wait_for_selector("body[data-mode='result']", state="attached", timeout=90000)
    head = page.inner_text("#x-head")
    check(head in (COPY["won"], COPY["lost"], COPY["drawn"]), f"the fight ends in a result ({head})")
    check(page.locator("#x-units .x-unit").count() == 3, "each of the three units is paid")
    check("+" in page.inner_text("#x-account"), "the account is paid")

    # a random fight, skipped
    page.click("#x-back")
    page.click("#campaign button.back")
    page.wait_for_selector("body[data-mode='menu']", state="attached", timeout=5000)
    page.click("#b-random")
    page.wait_for_selector("body[data-mode='battle']", state="attached", timeout=5000)
    page.click("#a-skip")
    page.wait_for_selector("body[data-mode='result']", state="attached", timeout=10000)
    check(page.locator("#x-units .x-unit").count() == 0, "a random fight pays no unit")
    check(page.inner_text("#x-note") == COPY["random_note"], "and says why")
    page.click("#x-back")

    # online, opened but not connected: nothing leaves the origin
    page.click("#b-online")
    page.wait_for_selector("body[data-mode='online']", state="attached", timeout=5000)
    check(page.is_visible("#o-host"), "the online screen offers to host")
    page.fill("#o-code", "!")
    page.click("#o-join")
    check(page.inner_text("#o-error") == COPY["online_bad_code"], "a code that cannot be one is refused")
    page.click("#o-back")

    # help
    page.click("#b-help")
    check(images_loaded(page, "#help"), "the triangle loaded")
    page.click("#help button.back")

    # progress survives a reload
    page.reload()
    page.wait_for_selector("body[data-mode='menu']", state="attached", timeout=10000)
    record = page.inner_text("#m-record")
    check(not record.startswith("0 won · 0 lost"), f"the save kept the fights played ({record})")
