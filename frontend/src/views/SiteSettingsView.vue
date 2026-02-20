<template>
  <section class="page-block">
    <header class="page-head">
      <div class="section-title">
        <p>{{ tr("管理员", "Admin") }}</p>
        <h1>{{ tr("站点设置", "Site Settings") }}</h1>
      </div>
      <div class="actions-row">
        <button class="btn-line" type="button" :disabled="loading" @click="loadSettings">
          {{ loading ? tr("刷新中...", "Refreshing...") : tr("刷新", "Refresh") }}
        </button>
      </div>
    </header>

    <article class="panel stack">
      <form class="form-grid" @submit.prevent="handleSubmit">
        <label>
          <span>{{ tr("站点名", "Site Name") }}</span>
          <input v-model.trim="form.site_name" maxlength="80" required />
        </label>
        <label>
          <span>{{ tr("站点副标题", "Site Subtitle") }}</span>
          <input v-model.trim="form.site_subtitle" maxlength="160" />
        </label>
        <label>
          <span>{{ tr("首页主标题", "Home Title") }}</span>
          <input v-model.trim="form.home_title" maxlength="160" required />
        </label>
        <label>
          <span>{{ tr("首页简介", "Home Tagline") }}</span>
          <textarea v-model.trim="form.home_tagline" rows="3" maxlength="2000" />
        </label>
        <label>
          <span>{{ tr("首页签名", "Home Signature") }}</span>
          <input v-model.trim="form.home_signature" maxlength="200" />
        </label>
        <label>
          <span>{{ tr("Footer 版权文案", "Footer Copyright") }}</span>
          <input v-model.trim="form.footer_text" maxlength="240" />
        </label>

        <button class="btn-solid" type="submit" :disabled="submitting">
          {{ submitting ? tr("保存中...", "Saving...") : tr("保存设置", "Save") }}
        </button>
      </form>

      <p v-if="updatedAt" class="soft">
        {{ tr("最后更新", "Last updated") }}: {{ formatTime(updatedAt) }}
      </p>
      <p v-if="error" class="error">{{ error }}</p>
    </article>
  </section>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";

import { ApiClientError, getAdminSiteSettings, updateAdminSiteSettings } from "../api/client";
import { useL10n } from "../composables/useL10n";
import { useAppStore } from "../stores/app";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

const { tr } = useL10n();
const authStore = useAuthStore();
const uiStore = useUiStore();
const appStore = useAppStore();

const loading = ref(false);
const submitting = ref(false);
const error = ref("");
const updatedAt = ref("");

const form = reactive({
  site_name: "",
  site_subtitle: "",
  home_title: "",
  home_tagline: "",
  home_signature: "",
  footer_text: ""
});

function accessTokenOrThrow() {
  const token = authStore.accessToken;
  if (!token) {
    throw new ApiClientError("access token missing", "unauthorized");
  }
  return token;
}

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

function applyForm(payload: {
  site_name: string;
  site_subtitle: string;
  home_title: string;
  home_tagline: string;
  home_signature: string;
  footer_text: string;
}) {
  form.site_name = payload.site_name;
  form.site_subtitle = payload.site_subtitle;
  form.home_title = payload.home_title;
  form.home_tagline = payload.home_tagline;
  form.home_signature = payload.home_signature;
  form.footer_text = payload.footer_text;
}

async function loadSettings() {
  loading.value = true;
  error.value = "";
  try {
    const data = await getAdminSiteSettings(accessTokenOrThrow());
    applyForm(data);
    updatedAt.value = data.updated_at;
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("加载失败", "Load failed");
  } finally {
    loading.value = false;
  }
}

async function handleSubmit() {
  submitting.value = true;
  error.value = "";
  try {
    const saved = await updateAdminSiteSettings(
      {
        site_name: form.site_name,
        site_subtitle: form.site_subtitle,
        home_title: form.home_title,
        home_tagline: form.home_tagline,
        home_signature: form.home_signature,
        footer_text: form.footer_text
      },
      accessTokenOrThrow()
    );
    applyForm(saved);
    updatedAt.value = saved.updated_at;
    appStore.applySiteSettings(saved);
    uiStore.success(tr("保存成功", "Saved"), tr("站点文案已更新。", "Site copy has been updated."), 2200);
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("保存失败", "Save failed");
    uiStore.error(tr("保存失败", "Save failed"), error.value);
  } finally {
    submitting.value = false;
  }
}

onMounted(async () => {
  await loadSettings();
});
</script>
