import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { getPublicSiteSettings, type SiteSettings } from "../api/client";
import {
  getLocaleHtmlLang,
  getLocaleOption,
  getNextLocale,
  I18N_DEFAULT_LOCALE,
  I18N_LOCALE_OPTIONS,
  isSupportedLocale,
  normalizeLocale
} from "../locales/i18n";

export type AppLocale = string;
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
  footer_text: "© 2026 Rust-CTF. All rights reserved.",
  time_display_mode: "utc"
};

const APP_PREFERENCES_STORAGE_KEY = "rust-ctf.preferences";

function isAppTheme(value: unknown): value is AppTheme {
  return value === "light" || value === "dark";
}

export const useAppStore = defineStore("app", () => {
  const backendBaseUrl = ref(import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080");
  const locale = ref<AppLocale>(I18N_DEFAULT_LOCALE);
  const theme = ref<AppTheme>("light");
  const hydrated = ref(false);
  const siteSettings = ref<SiteSettings>({ ...DEFAULT_SITE_SETTINGS });
  const siteSettingsLoaded = ref(false);

  const isDarkTheme = computed(() => theme.value === "dark");
  const isEnglish = computed(() => locale.value === "en");
  const localeOptions = computed(() => I18N_LOCALE_OPTIONS);
  const currentLocaleOption = computed(() => getLocaleOption(locale.value));
  const nextLocaleOption = computed(() => getLocaleOption(getNextLocale(locale.value)));

  function applyDocumentPreferences() {
    document.documentElement.setAttribute("data-theme", theme.value);
    document.documentElement.setAttribute("lang", getLocaleHtmlLang(locale.value));
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
      if (isSupportedLocale(parsed.locale)) {
        locale.value = normalizeLocale(parsed.locale);
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
    const normalized = normalizeLocale(nextLocale);
    if (locale.value === normalized) {
      return;
    }
    locale.value = normalized;
    persistPreferences();
    applyDocumentPreferences();
  }

  function toggleLocale() {
    setLocale(getNextLocale(locale.value));
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
    localeOptions,
    currentLocaleOption,
    nextLocaleOption,
    hydrateFromStorage,
    setLocale,
    toggleLocale,
    setTheme,
    toggleTheme,
    applySiteSettings,
    loadSiteSettings
  };
});
