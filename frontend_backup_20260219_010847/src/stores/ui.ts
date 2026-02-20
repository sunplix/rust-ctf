import { ref } from "vue";
import { defineStore } from "pinia";

export type ToastType = "success" | "error" | "info" | "warning";

export type ToastItem = {
  id: number;
  type: ToastType;
  title: string;
  message: string;
};

type PushToastPayload = {
  type: ToastType;
  title: string;
  message: string;
  durationMs?: number;
};

let nextToastId = 1;

export const useUiStore = defineStore("ui", () => {
  const toasts = ref<ToastItem[]>([]);

  function removeToast(id: number) {
    toasts.value = toasts.value.filter((item) => item.id !== id);
  }

  function pushToast(payload: PushToastPayload) {
    const durationMs = payload.durationMs ?? 3600;
    const id = nextToastId++;

    toasts.value = [
      ...toasts.value,
      {
        id,
        type: payload.type,
        title: payload.title,
        message: payload.message
      }
    ];

    if (durationMs > 0) {
      window.setTimeout(() => {
        removeToast(id);
      }, durationMs);
    }
  }

  function success(title: string, message: string, durationMs?: number) {
    pushToast({ type: "success", title, message, durationMs });
  }

  function error(title: string, message: string, durationMs?: number) {
    pushToast({ type: "error", title, message, durationMs });
  }

  function info(title: string, message: string, durationMs?: number) {
    pushToast({ type: "info", title, message, durationMs });
  }

  function warning(title: string, message: string, durationMs?: number) {
    pushToast({ type: "warning", title, message, durationMs });
  }

  return {
    toasts,
    removeToast,
    pushToast,
    success,
    error,
    info,
    warning
  };
});
