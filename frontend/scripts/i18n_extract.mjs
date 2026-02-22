#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(SCRIPT_DIR, "..");
const SRC_ROOT = path.join(FRONTEND_ROOT, "src");
const LOCALES_DIR = path.join(SRC_ROOT, "locales");

const CATALOG_PATH = path.join(LOCALES_DIR, "catalog.json");
const REVIEW_PATH = path.join(LOCALES_DIR, "catalog.review.csv");
const DYNAMIC_PATH = path.join(LOCALES_DIR, "dynamic_tr.review.csv");
const RUNTIME_PATH = path.join(LOCALES_DIR, "runtime.json");
const I18N_CONFIG_PATH = path.join(LOCALES_DIR, "i18n.config.json");

const SOURCE_EXTENSIONS = new Set([".vue", ".ts"]);
const SOURCE_IGNORED_DIRS = new Set(["assets", "locales"]);
const FALLBACK_I18N_CONFIG = {
  default_locale: "zh",
  fallback_locale: "en",
  supported_locales: [
    {
      code: "zh",
      label: "中文",
      short_label: "中文",
      html_lang: "zh-CN",
      date_locale: "zh-CN"
    },
    {
      code: "en",
      label: "English",
      short_label: "EN",
      html_lang: "en",
      date_locale: "en-US"
    }
  ]
};

function fnv1a32(input) {
  let hash = 0x811c9dc5;
  for (const char of input) {
    hash ^= char.codePointAt(0);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function messageKey(sourceZh, sourceEn) {
  return `m_${fnv1a32(`${sourceZh}\u0001${sourceEn}`)}`;
}

function decodeLiteral(quote, raw) {
  try {
    return Function(`"use strict"; return (${quote}${raw}${quote});`)();
  } catch {
    return raw;
  }
}

function normalizeText(input) {
  return input.replace(/\r\n/g, "\n").trim();
}

function lineNumberFromIndex(text, index) {
  let line = 1;
  for (let i = 0; i < index && i < text.length; i += 1) {
    if (text[i] === "\n") {
      line += 1;
    }
  }
  return line;
}

async function walkSourceFiles(dir, out = []) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SOURCE_IGNORED_DIRS.has(entry.name)) {
        continue;
      }
      await walkSourceFiles(fullPath, out);
      continue;
    }
    if (!entry.isFile()) {
      continue;
    }
    if (!SOURCE_EXTENSIONS.has(path.extname(entry.name))) {
      continue;
    }
    out.push(fullPath);
  }
  return out;
}

function extractStaticTrCalls(content, fileRel) {
  const results = [];
  const consumedRanges = [];
  const patterns = [
    /\btr\s*\(\s*"((?:\\.|[^"\\])*)"\s*,\s*"((?:\\.|[^"\\])*)"\s*\)/g,
    /\btr\s*\(\s*'((?:\\.|[^'\\])*)'\s*,\s*'((?:\\.|[^'\\])*)'\s*\)/g
  ];

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      if (match.index === undefined) {
        continue;
      }
      const quote = match[0].includes('"') ? '"' : "'";
      const sourceZh = normalizeText(decodeLiteral(quote, match[1] ?? ""));
      const sourceEn = normalizeText(decodeLiteral(quote, match[2] ?? ""));
      if (!sourceZh || !sourceEn) {
        continue;
      }
      const line = lineNumberFromIndex(content, match.index);
      results.push({
        sourceZh,
        sourceEn,
        source: `${fileRel}:${line}`
      });
      consumedRanges.push({
        start: match.index,
        end: match.index + match[0].length
      });
    }
  }

  return { results, consumedRanges };
}

function extractStaticTlCalls(content, fileRel) {
  const results = [];
  const patterns = [
    /\btl\s*\(\s*"((?:\\.|[^"\\])*)"\s*\)/g,
    /\btl\s*\(\s*'((?:\\.|[^'\\])*)'\s*\)/g
  ];

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      if (match.index === undefined) {
        continue;
      }
      const quote = match[0].includes('"') ? '"' : "'";
      const sourceZh = normalizeText(decodeLiteral(quote, match[1] ?? ""));
      if (!sourceZh) {
        continue;
      }
      const line = lineNumberFromIndex(content, match.index);
      results.push({ sourceZh, source: `${fileRel}:${line}` });
    }
  }

  return results;
}

function extractAdminTextMap(content, fileRel) {
  const blockMatch = content.match(/const\s+adminTextMap\s*:[^=]*=\s*\{([\s\S]*?)\}\s*;{2,}/);
  if (!blockMatch || blockMatch.index === undefined) {
    return [];
  }

  const blockText = blockMatch[1];
  const blockStart = blockMatch.index;
  const entryPattern = /"((?:\\.|[^"\\])*)"\s*:\s*"((?:\\.|[^"\\])*)"\s*,?/g;
  const results = [];

  for (const match of blockText.matchAll(entryPattern)) {
    if (match.index === undefined) {
      continue;
    }
    const sourceZh = normalizeText(decodeLiteral('"', match[1] ?? ""));
    const sourceEn = normalizeText(decodeLiteral('"', match[2] ?? ""));
    if (!sourceZh || !sourceEn) {
      continue;
    }
    const absoluteIndex = blockStart + match.index;
    const line = lineNumberFromIndex(content, absoluteIndex);
    results.push({
      sourceZh,
      sourceEn,
      source: `${fileRel}:${line}`
    });
  }

  return results;
}

function extractDynamicTrCalls(content, fileRel, consumedRanges) {
  const results = [];
  const trOpenPattern = /\btr\s*\(/g;

  for (const match of content.matchAll(trOpenPattern)) {
    if (match.index === undefined) {
      continue;
    }
    const index = match.index;
    const consumed = consumedRanges.some((range) => index >= range.start && index < range.end);
    if (consumed) {
      continue;
    }
    const line = lineNumberFromIndex(content, index);
    const snippet = content
      .slice(index, Math.min(content.length, index + 120))
      .replace(/\s+/g, " ")
      .trim();
    results.push({
      source: `${fileRel}:${line}`,
      snippet
    });
  }

  return results;
}

async function loadExistingCatalog() {
  try {
    const raw = await fs.readFile(CATALOG_PATH, "utf8");
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object") {
      return parsed;
    }
  } catch {
    // ignore missing/invalid file
  }
  return {};
}

function normalizeLocaleCode(value) {
  return typeof value === "string" ? value.trim().toLowerCase() : "";
}

async function loadI18nConfig() {
  let parsed = FALLBACK_I18N_CONFIG;
  try {
    const raw = await fs.readFile(I18N_CONFIG_PATH, "utf8");
    parsed = JSON.parse(raw);
  } catch {
    // use fallback config
  }

  const supportedLocales = [];
  const seen = new Set();
  for (const item of Array.isArray(parsed.supported_locales) ? parsed.supported_locales : []) {
    const code = normalizeLocaleCode(item?.code);
    if (!code || seen.has(code)) {
      continue;
    }
    seen.add(code);
    supportedLocales.push(code);
  }

  if (supportedLocales.length === 0) {
    supportedLocales.push("zh", "en");
  }

  const defaultLocaleRaw = normalizeLocaleCode(parsed.default_locale);
  const defaultLocale = supportedLocales.includes(defaultLocaleRaw)
    ? defaultLocaleRaw
    : supportedLocales[0];
  const fallbackLocaleRaw = normalizeLocaleCode(parsed.fallback_locale);
  const fallbackLocale = supportedLocales.includes(fallbackLocaleRaw)
    ? fallbackLocaleRaw
    : defaultLocale;

  return {
    supportedLocales,
    defaultLocale,
    fallbackLocale
  };
}

function defaultLocaleText(localeCode, entry, config) {
  if (localeCode === config.defaultLocale) {
    return entry.sourceZh;
  }
  if (localeCode === config.fallbackLocale) {
    return entry.sourceEn;
  }
  return entry.sourceEn;
}

function upsertEntry(catalog, entry, config, reviewFlag = false) {
  const key = messageKey(entry.sourceZh, entry.sourceEn);
  const existing = catalog[key] ?? {};

  const nextSources = new Set(Array.isArray(existing.sources) ? existing.sources : []);
  nextSources.add(entry.source);

  const nextLocaleValues = {};
  for (const localeCode of config.supportedLocales) {
    const existingValue =
      typeof existing[localeCode] === "string" && existing[localeCode].length > 0
        ? existing[localeCode]
        : "";
    nextLocaleValues[localeCode] = existingValue || defaultLocaleText(localeCode, entry, config);
  }

  catalog[key] = {
    source_zh: entry.sourceZh,
    source_en: entry.sourceEn,
    ...nextLocaleValues,
    review:
      typeof existing.review === "boolean"
        ? existing.review
        : reviewFlag,
    sources: Array.from(nextSources).sort()
  };
}

function toCsvRow(columns) {
  const escaped = columns.map((column) => {
    const value = String(column ?? "");
    if (value.includes('"') || value.includes(",") || value.includes("\n")) {
      return `"${value.replace(/"/g, '""')}"`;
    }
    return value;
  });
  return `${escaped.join(",")}\n`;
}

async function main() {
  const i18nConfig = await loadI18nConfig();
  const sourceFiles = await walkSourceFiles(SRC_ROOT);
  const existingCatalog = await loadExistingCatalog();
  const nextCatalog = {};

  const trEntries = [];
  const tlEntries = [];
  const adminMapEntries = [];
  const dynamicEntries = [];
  const zhToEnHints = new Map();

  for (const filePath of sourceFiles) {
    const fileRel = path.relative(FRONTEND_ROOT, filePath).replace(/\\/g, "/");
    const content = await fs.readFile(filePath, "utf8");

    const adminPairs = extractAdminTextMap(content, fileRel);
    for (const entry of adminPairs) {
      adminMapEntries.push(entry);
      if (!zhToEnHints.has(entry.sourceZh)) {
        zhToEnHints.set(entry.sourceZh, entry.sourceEn);
      }
    }

    const { results, consumedRanges } = extractStaticTrCalls(content, fileRel);
    for (const entry of results) {
      trEntries.push(entry);
      if (!zhToEnHints.has(entry.sourceZh)) {
        zhToEnHints.set(entry.sourceZh, entry.sourceEn);
      }
    }

    const tlResults = extractStaticTlCalls(content, fileRel);
    for (const entry of tlResults) {
      tlEntries.push(entry);
    }

    dynamicEntries.push(...extractDynamicTrCalls(content, fileRel, consumedRanges));
  }

  for (const entry of adminMapEntries) {
    upsertEntry(nextCatalog, entry, i18nConfig, false);
  }
  for (const entry of trEntries) {
    upsertEntry(nextCatalog, entry, i18nConfig, false);
  }
  for (const entry of tlEntries) {
    const hintedEn = zhToEnHints.get(entry.sourceZh) ?? entry.sourceZh;
    upsertEntry(
      nextCatalog,
      {
        sourceZh: entry.sourceZh,
        sourceEn: hintedEn,
        source: entry.source
      },
      i18nConfig,
      hintedEn === entry.sourceZh
    );
  }

  for (const [key, entry] of Object.entries(nextCatalog)) {
    const previous = existingCatalog[key];
    if (!previous || typeof previous !== "object") {
      continue;
    }

    for (const localeCode of i18nConfig.supportedLocales) {
      if (typeof previous[localeCode] === "string" && previous[localeCode].length > 0) {
        entry[localeCode] = previous[localeCode];
      }
    }
    if (typeof previous.review === "boolean") {
      entry.review = previous.review;
    }
  }

  const orderedCatalog = Object.fromEntries(
    Object.entries(nextCatalog)
      .sort((left, right) => left[0].localeCompare(right[0]))
      .map(([key, value]) => {
        const entry = {
          source_zh: value.source_zh,
          source_en: value.source_en
        };
        for (const localeCode of i18nConfig.supportedLocales) {
          entry[localeCode] = value[localeCode];
        }
        entry.review = Boolean(value.review);
        entry.sources = [...value.sources].sort();
        return [key, entry];
      })
  );

  await fs.mkdir(LOCALES_DIR, { recursive: true });
  await fs.writeFile(CATALOG_PATH, `${JSON.stringify(orderedCatalog, null, 2)}\n`, "utf8");

  const runtimePairs = {};
  const runtimeBySourceZh = {};
  const runtimeZhToEn = {};
  for (const [key, value] of Object.entries(orderedCatalog)) {
    const localeTextMap = {};
    for (const localeCode of i18nConfig.supportedLocales) {
      localeTextMap[localeCode] =
        typeof value[localeCode] === "string"
          ? value[localeCode]
          : defaultLocaleText(localeCode, { sourceZh: value.source_zh, sourceEn: value.source_en }, i18nConfig);
    }
    runtimePairs[key] = localeTextMap;
    if (!runtimeBySourceZh[value.source_zh]) {
      runtimeBySourceZh[value.source_zh] = { ...localeTextMap };
    }
    if (!runtimeZhToEn[value.source_zh]) {
      runtimeZhToEn[value.source_zh] =
        localeTextMap.en ??
        localeTextMap[i18nConfig.fallbackLocale] ??
        localeTextMap[i18nConfig.defaultLocale] ??
        value.source_en;
    }
  }

  await fs.writeFile(
    RUNTIME_PATH,
    `${JSON.stringify(
      {
        supported_locales: i18nConfig.supportedLocales,
        default_locale: i18nConfig.defaultLocale,
        fallback_locale: i18nConfig.fallbackLocale,
        pairs: runtimePairs,
        by_source_zh: runtimeBySourceZh,
        zh_to_en: runtimeZhToEn
      },
      null,
      2
    )}\n`,
    "utf8"
  );

  let reviewCsv = "";
  reviewCsv += toCsvRow([
    "key",
    "source_zh",
    "source_en",
    ...i18nConfig.supportedLocales,
    "review",
    "source_count",
    "sources"
  ]);
  for (const [key, value] of Object.entries(orderedCatalog)) {
    const localeColumns = i18nConfig.supportedLocales.map((localeCode) => value[localeCode] ?? "");
    reviewCsv += toCsvRow([
      key,
      value.source_zh,
      value.source_en,
      ...localeColumns,
      value.review ? "true" : "false",
      value.sources.length,
      value.sources.join(" | ")
    ]);
  }
  await fs.writeFile(REVIEW_PATH, reviewCsv, "utf8");

  const dedupDynamic = new Map();
  for (const item of dynamicEntries) {
    const indexKey = `${item.source}::${item.snippet}`;
    if (!dedupDynamic.has(indexKey)) {
      dedupDynamic.set(indexKey, item);
    }
  }

  let dynamicCsv = "";
  dynamicCsv += toCsvRow(["source", "snippet"]);
  for (const item of [...dedupDynamic.values()].sort((a, b) => a.source.localeCompare(b.source))) {
    dynamicCsv += toCsvRow([item.source, item.snippet]);
  }
  await fs.writeFile(DYNAMIC_PATH, dynamicCsv, "utf8");

  console.log(`[i18n] catalog entries: ${Object.keys(orderedCatalog).length}`);
  console.log(`[i18n] runtime json: ${path.relative(FRONTEND_ROOT, RUNTIME_PATH)}`);
  console.log(`[i18n] review csv: ${path.relative(FRONTEND_ROOT, REVIEW_PATH)}`);
  console.log(`[i18n] dynamic tr csv: ${path.relative(FRONTEND_ROOT, DYNAMIC_PATH)} (${dedupDynamic.size})`);
}

main().catch((error) => {
  console.error("[i18n] extraction failed:", error);
  process.exitCode = 1;
});
