import { computed } from "vue";

import { useAppStore } from "../stores/app";

export function useL10n() {
  const appStore = useAppStore();
  const locale = computed(() => appStore.locale);
  const isEnglish = computed(() => appStore.locale === "en");

  const tr = (zh: string, en: string) => {
    return isEnglish.value ? en : zh;
  };

  return {
    locale,
    isEnglish,
    tr
  };
}
