import { ACTIVE_MODES, type ActiveMode } from "./config.ts";

export type HistoryFilterDraft = {
  query?: string;
  from_day?: string;
  to_day?: string;
  mode?: string;
  app_id?: string;
  src_lang?: string;
};

export type ValidatedHistoryFilter = {
  query: string;
  from_day: string;
  to_day: string;
  mode: string;
  app_id: string;
  src_lang: string;
};

export type PreparedHistoryFilter =
  | {
      ok: true;
      filter: ValidatedHistoryFilter;
      isReversed: boolean;
    }
  | {
      ok: false;
      error: string;
    };

const DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/;

function isValidDayString(str: string): boolean {
  if (!DATE_REGEX.test(str)) return false;
  const [y, m, d] = str.split("-").map(Number);
  if (m < 1 || m > 12 || d < 1 || d > 31) return false;
  const date = new Date(Date.UTC(y, m - 1, d));
  return (
    date.getUTCFullYear() === y &&
    date.getUTCMonth() === m - 1 &&
    date.getUTCDate() === d
  );
}

export function prepareHistoryFilter(draft: HistoryFilterDraft): PreparedHistoryFilter {
  const query = (draft.query ?? "").trim();
  const from_day = (draft.from_day ?? "").trim();
  const to_day = (draft.to_day ?? "").trim();
  const mode = (draft.mode ?? "").trim();
  const app_id = (draft.app_id ?? "").trim();
  const src_lang = (draft.src_lang ?? "").trim();

  if (from_day.length > 0 && !isValidDayString(from_day)) {
    return { ok: false, error: `Invalid date: ${from_day}` };
  }
  if (to_day.length > 0 && !isValidDayString(to_day)) {
    return { ok: false, error: `Invalid date: ${to_day}` };
  }

  let isReversed = false;
  if (from_day.length > 0 && to_day.length > 0) {
    if (from_day > to_day) {
      isReversed = true;
    }
  }

  if (mode.length > 0 && !(ACTIVE_MODES as readonly string[]).includes(mode)) {
    return { ok: false, error: `Unknown mode: ${mode}` };
  }

  return {
    ok: true,
    filter: {
      query,
      from_day,
      to_day,
      mode,
      app_id,
      src_lang,
    },
    isReversed,
  };
}

export function isFilterNarrowed(
  filter: Omit<ValidatedHistoryFilter, "query">,
): boolean {
  return (
    filter.from_day.length > 0 ||
    filter.to_day.length > 0 ||
    filter.mode.length > 0 ||
    filter.app_id.length > 0 ||
    filter.src_lang.length > 0
  );
}

export function filterCacheKey(filter: ValidatedHistoryFilter): string {
  return [
    filter.query,
    filter.from_day,
    filter.to_day,
    filter.mode,
    filter.app_id,
    filter.src_lang,
  ].join("\0");
}
