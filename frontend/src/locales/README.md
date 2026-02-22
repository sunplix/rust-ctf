# Frontend i18n Workflow

## Goals

- Extract all `tr()` / `tl()` text pairs into a single catalog.
- Keep a review table for wording polish.
- Keep runtime fallback behavior safe if a catalog item is missing.

## Files

- `catalog.json`: editable catalog used at runtime.
- `runtime.json`: lightweight runtime dictionary generated from `catalog.json`.
- `catalog.review.csv`: review-friendly table (key + source + current copy + source locations).
- `dynamic_tr.review.csv`: dynamic `tr(...)` calls that cannot be fully extracted automatically.

## Usage

1. Re-extract texts:

```bash
npm run i18n:extract
```

2. Review wording:

- Open `src/locales/catalog.review.csv`.
- Filter `review=true` first to process pending entries.

3. Modify copy:

- Edit `src/locales/catalog.json` fields:
  - `zh`: Chinese display copy
  - `en`: English display copy

## Where To Edit (Author Workflow)

If you want frontend text to change, edit this file only:

- `src/locales/catalog.json`

Rules:

- Only change `zh` / `en` fields.
- Do not change `source_zh` / `source_en` (they are extraction anchors).
- Do not edit `runtime.json` manually (auto-generated).
- Keep placeholders such as `{username}`, `{maxSize}`, `{teamName}` unchanged.

Then regenerate runtime dictionary:

```bash
npm run i18n:extract
```

This updates:

- `src/locales/runtime.json` (used by runtime `tr()` / `tl()`)
- `src/locales/catalog.review.csv`
- `src/locales/dynamic_tr.review.csv`

## Notes

- `source_zh` / `source_en` are extraction anchors (do not use them as display copy fields).
- `runtime.json` is auto-generated; edit `catalog.json`, then rerun extraction.
- Runtime uses generated dictionary first, then falls back to inline `tr()` / `tl()` values.
- For entries in `dynamic_tr.review.csv`, keep using inline expressions or migrate manually to explicit keyed translations later.

## Add More Languages (beyond zh/en)

Current implementation is bilingual by design (`zh` + `en`).  
To add a third language (for example `ja`), update these parts:

1. Locale state and language switch

- Add new locale option in app store / UI switcher (currently `zh` / `en`).

2. Catalog and runtime schema

- Extend `catalog.json` item shape to include the new field (for example `ja`).
- Extend `runtime.json` generation in `scripts/i18n_extract.mjs` so each key includes new language text.

3. Runtime translator

- Update `src/composables/useL10n.ts`:
  - `tr()` should select by current locale (`zh` / `en` / `ja` ...).
  - `tl()` fallback map should support non-zh locales, or be replaced with a generic locale lookup.

4. Review export

- Add a new column in `catalog.review.csv` output for the new language to support copy review.

5. Fill translations and regenerate

```bash
npm run i18n:extract
```

If you want, we can do this refactor in code directly next (make i18n locale-agnostic instead of hardcoded zh/en).
