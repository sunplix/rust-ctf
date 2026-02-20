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

export type ConfirmDialogIntent = "default" | "danger";

export type ConfirmDialogItem = {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  intent: ConfirmDialogIntent;
};

export type PromptDialogItem = {
  title: string;
  message: string;
  inputLabel: string;
  placeholder: string;
  defaultValue: string;
  confirmLabel: string;
  cancelLabel: string;
  intent: ConfirmDialogIntent;
  required: boolean;
  maxLength: number;
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

type ConfirmDialogPayload = {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  intent?: ConfirmDialogIntent;
};

type PromptDialogPayload = {
  title: string;
  message: string;
  inputLabel?: string;
  placeholder?: string;
  defaultValue?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  intent?: ConfirmDialogIntent;
  required?: boolean;
  maxLength?: number;
};

let nextToastId = 1;
let nextAlertId = 1;

export const useUiStore = defineStore("ui", () => {
  const toasts = ref<ToastItem[]>([]);
  const alerts = ref<AlertItem[]>([]);
  const confirmDialog = ref<ConfirmDialogItem | null>(null);
  const promptDialog = ref<PromptDialogItem | null>(null);
  let confirmResolver: ((value: boolean) => void) | null = null;
  let promptResolver: ((value: string | null) => void) | null = null;

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

  function settleConfirm(result: boolean) {
    const resolver = confirmResolver;
    confirmResolver = null;
    confirmDialog.value = null;
    resolver?.(result);
  }

  function settlePrompt(result: string | null) {
    const resolver = promptResolver;
    promptResolver = null;
    promptDialog.value = null;
    resolver?.(result);
  }

  function confirm(payload: ConfirmDialogPayload) {
    if (promptResolver) {
      promptResolver(null);
      promptResolver = null;
      promptDialog.value = null;
    }
    if (confirmResolver) {
      confirmResolver(false);
      confirmResolver = null;
    }

    confirmDialog.value = {
      title: payload.title,
      message: payload.message,
      confirmLabel: payload.confirmLabel ?? "Confirm",
      cancelLabel: payload.cancelLabel ?? "Cancel",
      intent: payload.intent ?? "default"
    };

    return new Promise<boolean>((resolve) => {
      confirmResolver = resolve;
    });
  }

  function prompt(payload: PromptDialogPayload) {
    if (confirmResolver) {
      confirmResolver(false);
      confirmResolver = null;
      confirmDialog.value = null;
    }
    if (promptResolver) {
      promptResolver(null);
      promptResolver = null;
    }

    promptDialog.value = {
      title: payload.title,
      message: payload.message,
      inputLabel: payload.inputLabel ?? "",
      placeholder: payload.placeholder ?? "",
      defaultValue: payload.defaultValue ?? "",
      confirmLabel: payload.confirmLabel ?? "Confirm",
      cancelLabel: payload.cancelLabel ?? "Cancel",
      intent: payload.intent ?? "default",
      required: payload.required ?? true,
      maxLength: payload.maxLength ?? 200
    };

    return new Promise<string | null>((resolve) => {
      promptResolver = resolve;
    });
  }

  function confirmAccept() {
    settleConfirm(true);
  }

  function confirmCancel() {
    settleConfirm(false);
  }

  function promptAccept(value: string) {
    settlePrompt(value);
  }

  function promptCancel() {
    settlePrompt(null);
  }

  return {
    toasts,
    alerts,
    confirmDialog,
    promptDialog,
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
    clearAlerts,
    confirm,
    confirmAccept,
    confirmCancel,
    prompt,
    promptAccept,
    promptCancel
  };
});
