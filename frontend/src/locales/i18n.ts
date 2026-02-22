import rawI18nConfig from "./i18n.config.json";

export type LocaleOption = {
  code: string;
  label: string;
  shortLabel: string;
  htmlLang: string;
  dateLocale: string;
};

type RawLocaleOption = {
  code?: unknown;
  label?: unknown;
  short_label?: unknown;
  html_lang?: unknown;
  date_locale?: unknown;
};

type RawI18nConfig = {
  default_locale?: unknown;
  fallback_locale?: unknown;
  supported_locales?: unknown;
};

const FALLBACK_LOCALE_OPTIONS: LocaleOption[] = [
  {
    code: "zh",
    label: "中文",
    shortLabel: "中文",
    htmlLang: "zh-CN",
    dateLocale: "zh-CN"
  },
  {
    code: "en",
    label: "English",
    shortLabel: "EN",
    htmlLang: "en",
    dateLocale: "en-US"
  }
];

function normalizeLocaleCode(value: unknown): string {
  return typeof value === "string" ? value.trim().toLowerCase() : "";
}

function normalizeLocaleOption(value: unknown): LocaleOption | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const item = value as RawLocaleOption;
  const code = normalizeLocaleCode(item.code);
  if (!code) {
    return null;
  }
  const label = typeof item.label === "string" && item.label.trim().length > 0 ? item.label.trim() : code;
  const shortLabel =
    typeof item.short_label === "string" && item.short_label.trim().length > 0
      ? item.short_label.trim()
      : label;
  const htmlLang =
    typeof item.html_lang === "string" && item.html_lang.trim().length > 0
      ? item.html_lang.trim()
      : code;
  const dateLocale =
    typeof item.date_locale === "string" && item.date_locale.trim().length > 0
      ? item.date_locale.trim()
      : htmlLang;

  return {
    code,
    label,
    shortLabel,
    htmlLang,
    dateLocale
  };
}

function buildLocaleOptions(raw: unknown): LocaleOption[] {
  if (!Array.isArray(raw)) {
    return [...FALLBACK_LOCALE_OPTIONS];
  }
  const out: LocaleOption[] = [];
  const seen = new Set<string>();
  for (const item of raw) {
    const normalized = normalizeLocaleOption(item);
    if (!normalized || seen.has(normalized.code)) {
      continue;
    }
    seen.add(normalized.code);
    out.push(normalized);
  }
  if (out.length === 0) {
    return [...FALLBACK_LOCALE_OPTIONS];
  }
  return out;
}

const parsedConfig = rawI18nConfig as RawI18nConfig;
const options = buildLocaleOptions(parsedConfig.supported_locales);
const optionMap = new Map(options.map((item) => [item.code, item]));

function resolveLocaleCode(value: unknown, fallback: string): string {
  const code = normalizeLocaleCode(value);
  return optionMap.has(code) ? code : fallback;
}

const defaultLocale = resolveLocaleCode(parsedConfig.default_locale, options[0].code);
const fallbackLocale = resolveLocaleCode(parsedConfig.fallback_locale, defaultLocale);

export const I18N_LOCALE_OPTIONS = options;
export const I18N_LOCALE_CODES = options.map((item) => item.code);
export const I18N_DEFAULT_LOCALE = defaultLocale;
export const I18N_FALLBACK_LOCALE = fallbackLocale;

export function isSupportedLocale(value: unknown): value is string {
  return optionMap.has(normalizeLocaleCode(value));
}

export function normalizeLocale(value: unknown): string {
  const code = normalizeLocaleCode(value);
  return optionMap.has(code) ? code : I18N_DEFAULT_LOCALE;
}

export function getLocaleOption(value: unknown): LocaleOption {
  const code = normalizeLocale(value);
  return optionMap.get(code) ?? options[0];
}

export function getNextLocale(value: unknown): string {
  const code = normalizeLocale(value);
  const index = I18N_LOCALE_CODES.findIndex((item) => item === code);
  if (index < 0 || I18N_LOCALE_CODES.length === 0) {
    return I18N_DEFAULT_LOCALE;
  }
  return I18N_LOCALE_CODES[(index + 1) % I18N_LOCALE_CODES.length];
}

export function getLocaleDateLocale(value: unknown): string {
  return getLocaleOption(value).dateLocale;
}

export function getLocaleHtmlLang(value: unknown): string {
  return getLocaleOption(value).htmlLang;
}
