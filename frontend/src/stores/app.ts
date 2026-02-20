import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { getPublicSiteSettings, type SiteSettings } from "../api/client";

export type AppLocale = "zh" | "en";
export type AppTheme = "light" | "dark";

type AppPreferences = {
  locale: AppLocale;
  theme: AppTheme;
};

const DEFAULT_SITE_SETTINGS: SiteSettings = {
  site_name: "RUST CTF",
  site_subtitle: "竞赛平台",
  home_title: "欢迎来到 Rust CTF",
  home_tagline: "面向实战的安全竞赛平台，专注比赛与协作。",
  home_signature: "Think clearly. Ship securely.",
  footer_text: "© 2026 Rust-CTF. All rights reserved."
};

const APP_PREFERENCES_STORAGE_KEY = "rust-ctf.preferences";

function isAppLocale(value: unknown): value is AppLocale {
  return value === "zh" || value === "en";
}

function isAppTheme(value: unknown): value is AppTheme {
  return value === "light" || value === "dark";
}

export const useAppStore = defineStore("app", () => {
  const backendBaseUrl = ref(import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080");
  const locale = ref<AppLocale>("zh");
  const theme = ref<AppTheme>("light");
  const hydrated = ref(false);
  const siteSettings = ref<SiteSettings>({ ...DEFAULT_SITE_SETTINGS });
  const siteSettingsLoaded = ref(false);

  const isDarkTheme = computed(() => theme.value === "dark");
  const isEnglish = computed(() => locale.value === "en");

  function applyDocumentPreferences() {
    document.documentElement.setAttribute("data-theme", theme.value);
    document.documentElement.setAttribute("lang", locale.value === "zh" ? "zh-CN" : "en");
  }

  function persistPreferences() {
    const payload: AppPreferences = {
      locale: locale.value,
      theme: theme.value
    };
    localStorage.setItem(APP_PREFERENCES_STORAGE_KEY, JSON.stringify(payload));
  }

  function hydrateFromStorage() {
    if (hydrated.value) {
      return;
    }

    hydrated.value = true;
    const raw = localStorage.getItem(APP_PREFERENCES_STORAGE_KEY);
    if (!raw) {
      applyDocumentPreferences();
      return;
    }

    try {
      const parsed = JSON.parse(raw) as Partial<AppPreferences>;
      if (isAppLocale(parsed.locale)) {
        locale.value = parsed.locale;
      }
      if (isAppTheme(parsed.theme)) {
        theme.value = parsed.theme;
      }
    } catch {
      localStorage.removeItem(APP_PREFERENCES_STORAGE_KEY);
    }

    applyDocumentPreferences();
  }

  function setLocale(nextLocale: AppLocale) {
    if (locale.value === nextLocale) {
      return;
    }
    locale.value = nextLocale;
    persistPreferences();
    applyDocumentPreferences();
  }

  function toggleLocale() {
    setLocale(locale.value === "zh" ? "en" : "zh");
  }

  function setTheme(nextTheme: AppTheme) {
    if (theme.value === nextTheme) {
      return;
    }
    theme.value = nextTheme;
    persistPreferences();
    applyDocumentPreferences();
  }

  function toggleTheme() {
    setTheme(theme.value === "light" ? "dark" : "light");
  }

  function applySiteSettings(nextSettings: Partial<SiteSettings>) {
    siteSettings.value = {
      ...siteSettings.value,
      ...nextSettings
    };
  }

  async function loadSiteSettings(force = false) {
    if (siteSettingsLoaded.value && !force) {
      return;
    }

    try {
      const remote = await getPublicSiteSettings();
      applySiteSettings(remote);
    } catch {
      applySiteSettings(DEFAULT_SITE_SETTINGS);
    } finally {
      siteSettingsLoaded.value = true;
    }
  }

  return {
    backendBaseUrl,
    locale,
    theme,
    hydrated,
    siteSettings,
    siteSettingsLoaded,
    isDarkTheme,
    isEnglish,
    hydrateFromStorage,
    setLocale,
    toggleLocale,
    setTheme,
    toggleTheme,
    applySiteSettings,
    loadSiteSettings
  };
});
