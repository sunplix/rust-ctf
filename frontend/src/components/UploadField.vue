<template>
  <div class="upload-field" :class="{ 'is-filled': !!modelValue, 'is-disabled': disabled }">
    <input
      ref="nativeInput"
      :key="inputKey"
      class="upload-field-native"
      type="file"
      :accept="accept || undefined"
      :disabled="disabled"
      @change="handleNativeChange"
    />

    <button class="ghost upload-field-trigger" type="button" :disabled="disabled" @click="openPicker">
      {{ buttonLabel }}
    </button>

    <div class="upload-field-meta" aria-live="polite">
      <p class="upload-field-name">{{ currentFileName }}</p>
      <p class="upload-field-hint">{{ currentHint }}</p>
    </div>

    <button
      v-if="modelValue"
      class="ghost upload-field-clear"
      type="button"
      :disabled="disabled"
      @click="clearFile"
    >
      {{ clearLabel }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: File | null;
    inputKey?: string | number;
    accept?: string;
    disabled?: boolean;
    buttonLabel?: string;
    clearLabel?: string;
    placeholder?: string;
    hint?: string;
  }>(),
  {
    modelValue: null,
    inputKey: 0,
    accept: "",
    disabled: false,
    buttonLabel: "Select file",
    clearLabel: "Clear",
    placeholder: "No file selected",
    hint: ""
  }
);

const emit = defineEmits<{
  (event: "update:modelValue", value: File | null): void;
  (event: "change", value: File | null): void;
}>();

const nativeInput = ref<HTMLInputElement | null>(null);

const currentFileName = computed(() => {
  return props.modelValue?.name || props.placeholder;
});

const currentHint = computed(() => {
  if (!props.modelValue) {
    return props.hint;
  }

  if (!Number.isFinite(props.modelValue.size)) {
    return "";
  }

  const size = props.modelValue.size;
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
});

function openPicker() {
  if (props.disabled) {
    return;
  }
  nativeInput.value?.click();
}

function handleNativeChange(event: Event) {
  const target = event.target as HTMLInputElement | null;
  const file = target?.files?.[0] ?? null;
  emit("update:modelValue", file);
  emit("change", file);
}

function clearFile() {
  emit("update:modelValue", null);
  emit("change", null);
  if (nativeInput.value) {
    nativeInput.value.value = "";
  }
}
</script>

<style scoped>
.upload-field {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 0.62rem;
  min-height: 3.24rem;
  padding: 0.5rem 0.58rem;
  border-radius: calc(var(--radius-md) + 2px);
  background: color-mix(in srgb, var(--glass-mid) 88%, transparent 12%);
  box-shadow:
    inset 0 0 0 1px var(--line-mid),
    inset 0 -1px 0 var(--line-soft);
}

.upload-field.is-filled {
  background: color-mix(in srgb, var(--glass-soft) 84%, transparent 16%);
}

.upload-field.is-disabled {
  opacity: 0.66;
  pointer-events: none;
}

.upload-field-native {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
}

.upload-field-trigger {
  min-height: 2.08rem;
  padding-inline: 1rem;
  white-space: nowrap;
}

.upload-field-meta {
  min-width: 0;
  display: grid;
  gap: 0.2rem;
}

.upload-field-name,
.upload-field-hint {
  margin: 0;
}

.upload-field-name {
  font-weight: 600;
  color: var(--fg-0);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.upload-field-hint {
  font-size: 0.84rem;
  color: var(--fg-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.upload-field-clear {
  min-height: 2.08rem;
  min-width: 4.4rem;
  white-space: nowrap;
}

@media (max-width: 680px) {
  .upload-field {
    grid-template-columns: 1fr;
    align-items: stretch;
  }

  .upload-field-trigger,
  .upload-field-clear {
    width: 100%;
  }
}
</style>
