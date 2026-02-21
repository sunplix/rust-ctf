import { computed } from "vue";

import { useAppStore } from "../stores/app";
import { useL10n } from "./useL10n";

export function useTimeFormat() {
  const appStore = useAppStore();
  const { locale } = useL10n();

  const localeTag = computed(() => (locale.value === "en" ? "en-US" : "zh-CN"));
  const isUtcMode = computed(() => appStore.siteSettings.time_display_mode === "utc");

  function formatTime(
    input: string | number | Date,
    options?: Intl.DateTimeFormatOptions
  ): string {
    const date = input instanceof Date ? input : new Date(input);
    if (Number.isNaN(date.getTime())) {
      return typeof input === "string" ? input : "";
    }

    const formatted = date.toLocaleString(localeTag.value, {
      ...(isUtcMode.value ? { timeZone: "UTC" } : {}),
      ...options
    });

    return isUtcMode.value ? `${formatted} UTC` : formatted;
  }

  function formatTimeOnly(
    input: string | number | Date,
    options?: Intl.DateTimeFormatOptions
  ): string {
    const date = input instanceof Date ? input : new Date(input);
    if (Number.isNaN(date.getTime())) {
      return typeof input === "string" ? input : "";
    }

    const formatted = date.toLocaleTimeString(localeTag.value, {
      ...(isUtcMode.value ? { timeZone: "UTC" } : {}),
      hour: "2-digit",
      minute: "2-digit",
      ...options
    });

    return isUtcMode.value ? `${formatted} UTC` : formatted;
  }

  return {
    isUtcMode,
    formatTime,
    formatTimeOnly
  };
}
