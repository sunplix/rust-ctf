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
  - other locale fields if configured (for example `ja`)

## Where To Edit (Author Workflow)

If you want frontend text to change, edit this file only:

- `src/locales/catalog.json`

Rules:

- Only change language fields (for example `zh` / `en` / `ja`).
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

Current runtime is locale-config driven.

1. Edit locale config

```bash
src/locales/i18n.config.json
```

- Add one item under `supported_locales`:
  - `code`: locale id (for example `ja`)
  - `label`: full label
  - `short_label`: short button label shown in top bar
  - `html_lang`: HTML `lang` value
  - `date_locale`: `Intl.DateTimeFormat` locale tag
- Ensure `default_locale` and `fallback_locale` are valid locale codes in `supported_locales`.

2. Regenerate catalog/runtime

```bash
npm run i18n:extract
```

3. Fill translation text

- Open `src/locales/catalog.json`.
- New locale field (for example `ja`) will appear in each entry.
- Fill translated copy for that field.

4. Regenerate again after edits

```bash
npm run i18n:extract
```

5. Verify in UI

- Use top bar language switch button to cycle locales.
- Check date/time formatting and key pages for missing copy.
