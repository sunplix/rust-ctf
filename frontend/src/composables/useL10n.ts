import { computed } from "vue";

import { useAppStore } from "../stores/app";
import rawRuntime from "../locales/runtime.json";

type RuntimePair = {
  zh: string;
  en: string;
};

type RuntimeCatalog = {
  pairs: Record<string, RuntimePair>;
  zh_to_en: Record<string, string>;
};

const runtimeCatalog = rawRuntime as RuntimeCatalog;

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

export function useL10n() {
  const appStore = useAppStore();
  const locale = computed(() => appStore.locale);
  const isEnglish = computed(() => appStore.locale === "en");

  const tr = (zh: string, en: string) => {
    const sourceZh = normalizeSourceText(zh);
    const sourceEn = normalizeSourceText(en);
    const key = catalogKey(sourceZh, sourceEn);
    const entry = runtimeCatalog.pairs[key];
    const zhText = entry?.zh ?? zh;
    const enText = entry?.en ?? en;
    return isEnglish.value ? enText : zhText;
  };

  const tl = (zh: string) => {
    if (!isEnglish.value) {
      return zh;
    }

    const sourceZh = normalizeSourceText(zh);
    const exact = runtimeCatalog.zh_to_en[sourceZh];
    if (exact) {
      return exact;
    }

    const fallback = runtimeCatalog.pairs[catalogKey(sourceZh, sourceZh)];
    return fallback?.en ?? zh;
  };

  return {
    locale,
    isEnglish,
    tr,
    tl
  };
}
