import { computed } from "vue";

import { useAppStore } from "../stores/app";
import rawRuntime from "../locales/runtime.json";
import {
  I18N_DEFAULT_LOCALE,
  I18N_FALLBACK_LOCALE,
  isSupportedLocale,
  normalizeLocale
} from "../locales/i18n";

type RuntimePair = Record<string, string>;

type RuntimeCatalog = {
  supported_locales?: string[];
  default_locale?: string;
  fallback_locale?: string;
  pairs: Record<string, RuntimePair>;
  by_source_zh?: Record<string, RuntimePair>;
  zh_to_en?: Record<string, string>;
};

const runtimeCatalog = rawRuntime as RuntimeCatalog;
const runtimeDefaultLocale =
  typeof runtimeCatalog.default_locale === "string" && isSupportedLocale(runtimeCatalog.default_locale)
    ? runtimeCatalog.default_locale
    : I18N_DEFAULT_LOCALE;
const runtimeFallbackLocale =
  typeof runtimeCatalog.fallback_locale === "string" && isSupportedLocale(runtimeCatalog.fallback_locale)
    ? runtimeCatalog.fallback_locale
    : I18N_FALLBACK_LOCALE;

function fnv1a32(input: string): string {
  let hash = 0x811c9dc5;
  for (const char of input) {
    const codePoint = char.codePointAt(0) ?? 0;
    hash ^= codePoint;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function normalizeSourceText(text: string): string {
  return text.replace(/\r\n/g, "\n").trim();
}

function catalogKey(sourceZh: string, sourceEn: string): string {
  return `m_${fnv1a32(`${sourceZh}\u0001${sourceEn}`)}`;
}

function pickTranslation(
  translations: RuntimePair | undefined,
  locale: string,
  sourceFallbacks: Record<string, string>
): string {
  const localeText = translations?.[locale] ?? sourceFallbacks[locale];
  if (typeof localeText === "string" && localeText.length > 0) {
    return localeText;
  }

  const fallbackText = translations?.[runtimeFallbackLocale] ?? sourceFallbacks[runtimeFallbackLocale];
  if (typeof fallbackText === "string" && fallbackText.length > 0) {
    return fallbackText;
  }

  const defaultText = translations?.[runtimeDefaultLocale] ?? sourceFallbacks[runtimeDefaultLocale];
  if (typeof defaultText === "string" && defaultText.length > 0) {
    return defaultText;
  }

  const firstSource = Object.values(sourceFallbacks).find(
    (value) => typeof value === "string" && value.length > 0
  );
  if (firstSource) {
    return firstSource;
  }

  const firstTranslation = translations
    ? Object.values(translations).find((value) => typeof value === "string" && value.length > 0)
    : "";
  return firstTranslation ?? "";
}

export function useL10n() {
  const appStore = useAppStore();
  const locale = computed(() => appStore.locale);
  const normalizedLocale = computed(() => normalizeLocale(appStore.locale));
  const isEnglish = computed(() => normalizedLocale.value === "en");

  const tr = (zh: string, en: string) => {
    const sourceZh = normalizeSourceText(zh);
    const sourceEn = normalizeSourceText(en);
    const key = catalogKey(sourceZh, sourceEn);
    const entry = runtimeCatalog.pairs[key];
    return pickTranslation(entry, normalizedLocale.value, {
      zh: sourceZh,
      en: sourceEn
    });
  };

  const tl = (zh: string) => {
    const sourceZh = normalizeSourceText(zh);
    const bySource = runtimeCatalog.by_source_zh?.[sourceZh];
    const enHint =
      typeof runtimeCatalog.zh_to_en?.[sourceZh] === "string"
        ? runtimeCatalog.zh_to_en[sourceZh]
        : sourceZh;
    return pickTranslation(bySource, normalizedLocale.value, {
      zh: sourceZh,
      en: enHint
    });
  };

  return {
    locale,
    isEnglish,
    tr,
    tl
  };
}
