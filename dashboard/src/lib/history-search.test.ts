import assert from "node:assert/strict";
import test from "node:test";
import {
  filterCacheKey,
  isFilterNarrowed,
  prepareHistoryFilter,
  type HistoryFilterDraft,
} from "./history-search.ts";

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
