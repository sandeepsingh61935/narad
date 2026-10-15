import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  filterCacheKey,
  isFilterNarrowed,
  prepareHistoryFilter,
  type HistoryFilterDraft,
} from "./history-search.ts";

const here = dirname(fileURLToPath(import.meta.url));
const pageSource = readFileSync(join(here, "../routes/+page.svelte"), "utf8");


test("prepareHistoryFilter trims all fields and keeps valid dates and modes", () => {
  const draft: HistoryFilterDraft = {
    query: "  meeting notes  ",
    from_day: " 2026-10-01 ",
    to_day: " 2026-10-15 ",
    mode: " email ",
    app_id: " org.gnome.TextEditor ",
    src_lang: " en ",
  };
  const res = prepareHistoryFilter(draft);
  assert.equal(res.ok, true);
  if (res.ok) {
    assert.deepEqual(res.filter, {
      query: "meeting notes",
      from_day: "2026-10-01",
      to_day: "2026-10-15",
      mode: "email",
      app_id: "org.gnome.TextEditor",
      src_lang: "en",
    });
    assert.equal(res.isReversed, false);
  }
});

test("prepareHistoryFilter flags invalid date format", () => {
  const res = prepareHistoryFilter({
    query: "",
    from_day: "10/01/2026",
    to_day: "",
    mode: "",
    app_id: "",
    src_lang: "",
  });
  assert.equal(res.ok, false);
  if (!res.ok) {
    assert.match(res.error, /invalid date/i);
  }
});

test("prepareHistoryFilter detects reversed date range without throwing", () => {
  const res = prepareHistoryFilter({
    query: "",
    from_day: "2026-10-15",
    to_day: "2026-10-01",
    mode: "",
    app_id: "",
    src_lang: "",
  });
  assert.equal(res.ok, true);
  if (res.ok) {
    assert.equal(res.isReversed, true);
  }
});

test("prepareHistoryFilter rejects unknown mode", () => {
  const res = prepareHistoryFilter({
    query: "",
    from_day: "",
    to_day: "",
    mode: "supermode",
    app_id: "",
    src_lang: "",
  });
  assert.equal(res.ok, false);
  if (!res.ok) {
    assert.match(res.error, /unknown mode/i);
  }
});

test("isFilterNarrowed detects any active filter field", () => {
  assert.equal(
    isFilterNarrowed({
      from_day: "",
      to_day: "",
      mode: "",
      app_id: "",
      src_lang: "",
    }),
    false,
  );
  assert.equal(
    isFilterNarrowed({
      from_day: "2026-10-01",
      to_day: "",
      mode: "",
      app_id: "",
      src_lang: "",
    }),
    true,
  );
  assert.equal(
    isFilterNarrowed({
      from_day: "",
      to_day: "",
      mode: "code",
      app_id: "",
      src_lang: "",
    }),
    true,
  );
});

test("filterCacheKey generates unique key for cache busting", () => {
  const key1 = filterCacheKey({
    query: "hello",
    from_day: "2026-10-01",
    to_day: "2026-10-02",
    mode: "plain",
    app_id: "gedit",
    src_lang: "en",
  });
  const key2 = filterCacheKey({
    query: "hello",
    from_day: "2026-10-01",
    to_day: "2026-10-03",
    mode: "plain",
    app_id: "gedit",
    src_lang: "en",
  });
  assert.notEqual(key1, key2);
});

test("history page uses prepareHistoryFilter and active mode options", () => {
  assert.match(pageSource, /prepareHistoryFilter/);
  assert.match(pageSource, /ACTIVE_MODES/);
  assert.match(pageSource, /All modes/);
});

