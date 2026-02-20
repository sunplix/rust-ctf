import { ref } from "vue";
import { defineStore } from "pinia";

export type ToastType = "success" | "error" | "info" | "warning";

export type ToastItem = {
  id: number;
  type: ToastType;
  title: string;
  message: string;
};

export type AlertItem = {
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

type PushAlertPayload = {
  type: ToastType;
  title: string;
  message: string;
};

let nextToastId = 1;
let nextAlertId = 1;

export const useUiStore = defineStore("ui", () => {
  const toasts = ref<ToastItem[]>([]);
  const alerts = ref<AlertItem[]>([]);

  function removeToast(id: number) {
    toasts.value = toasts.value.filter((item) => item.id !== id);
  }

  function removeAlert(id: number) {
    alerts.value = alerts.value.filter((item) => item.id !== id);
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

  function pushAlert(payload: PushAlertPayload) {
    const id = nextAlertId++;
    alerts.value = [
      ...alerts.value,
      {
        id,
        type: payload.type,
        title: payload.title,
        message: payload.message
      }
    ];
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

  function alertInfo(title: string, message: string) {
    pushAlert({ type: "info", title, message });
  }

  function alertError(title: string, message: string) {
    pushAlert({ type: "error", title, message });
  }

  function alertWarning(title: string, message: string) {
    pushAlert({ type: "warning", title, message });
  }

  function clearAlerts() {
    alerts.value = [];
  }

  return {
    toasts,
    alerts,
    removeToast,
    removeAlert,
    pushToast,
    pushAlert,
    success,
    error,
    info,
    warning,
    alertInfo,
    alertError,
    alertWarning,
    clearAlerts
  };
});
