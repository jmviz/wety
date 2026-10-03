// When each language was spoken (`/periods`), shown on tree nodes behind the
// "dates" toggle. Fetched once, the first time the toggle is switched on.
// Periods are keyed by exact language: an etymology-only variety (e.g. Late
// Latin) has no period of its own unless the server sends one.

import { createSignal } from "solid-js";
import { Lang } from "../search/types";

export type Basis = "conventional" | "attested" | "estimated";

export interface YearRange {
  earliest: number | null;
  latest: number | null;
}

export interface PeriodEvidence {
  claim_id: string;
  source: string;
  role: string;
  event: string;
  citation: string | null;
  url: string | null;
  quote: string | null;
  locator: string | null;
}

export interface Alternative extends PeriodEvidence {
  basis: Basis;
  label: string | null;
}

export interface LangPeriod {
  code: string;
  lang: Lang;
  kind: "stage" | "attested" | "proto" | "living";
  spoken: { start: YearRange | null; end: YearRange | null; ongoing: boolean };
  basis: Basis;
  label: string;
  evidence: PeriodEvidence[];
  alternatives: Alternative[];
}

export const BASIS_MARK: Record<Basis, string> = {
  conventional: "conv",
  attested: "att",
  estimated: "est",
};

export const BASIS_NAME: Record<Basis, string> = {
  conventional: "conventional period",
  attested: "attestation window",
  estimated: "scholarly estimate",
};

const [showDates, setShowDates] = createSignal(false);
const [periods, setPeriods] = createSignal<Map<number, LangPeriod>>(new Map());
let requested = false;

export { showDates };

export function toggleDates() {
  const on = !showDates();
  setShowDates(on);
  if (!on || requested) return;
  requested = true;
  fetch(`${import.meta.env.VITE_API_BASE_URL}/periods`)
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((body: { languages: LangPeriod[] }) =>
      setPeriods(new Map(body.languages.map((p) => [p.lang.id, p])))
    )
    .catch(() => {
      requested = false;
    });
}

export const periodCount = () => periods().size;

export function periodOf(langId: number): LangPeriod | undefined {
  return showDates() ? periods().get(langId) : undefined;
}
