<template>
  <section class="page-layout login-shell">
    <aside class="surface surface-dashed stack auth-hero">
      <header class="hero-head">
        <div class="hero-mark">
          <span class="mono">RC</span>
        </div>
        <div class="hero-heading">
          <p class="showcase-eyebrow">{{ tr("账户入口", "Account Entry") }}</p>
          <h1>{{ tr("欢迎登录", "Welcome Back") }}</h1>
        </div>
      </header>

      <p class="muted hero-intro">
        {{
          tr(
            "登录后即可进入比赛与队伍空间。注册支持邮箱激活，找回密码支持邮箱重置。",
            "Sign in to access contests and team space. Registration supports email activation and password recovery supports email reset."
          )
        }}
      </p>

      <div class="hero-points">
        <article class="hero-point">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3l7 3v6c0 4.3-2.8 7.8-7 9-4.2-1.2-7-4.7-7-9V6l7-3Z" />
            <path d="m9 12 2 2 4-4" />
          </svg>
          <div>
            <h3>{{ tr("邮箱激活", "Email Activation") }}</h3>
            <p class="soft">{{ tr("注册后通过邮件完成激活。", "Complete activation via email after registration.") }}</p>
          </div>
        </article>
        <article class="hero-point">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M3.8 12.2h16.4" />
            <path d="M8.2 7.4 3.8 12l4.4 4.6" />
            <path d="M15.8 7.4 20.2 12l-4.4 4.6" />
            <path d="M12.7 6 11.3 18" />
          </svg>
          <div>
            <h3>{{ tr("密码强度检测", "Password Strength Check") }}</h3>
            <p class="soft">{{ tr("实时反馈密码强度与规则达成情况。", "Realtime feedback for password strength and policy checks.") }}</p>
          </div>
        </article>
        <article class="hero-point">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <rect x="4" y="4" width="16" height="16" rx="3" />
            <path d="M8 9h8" />
            <path d="M8 12h8" />
            <path d="M8 15h5" />
          </svg>
          <div>
            <h3>{{ tr("邮箱找回", "Email Recovery") }}</h3>
            <p class="soft">{{ tr("忘记密码时可直接发起重置流程。", "Start reset directly when you forget your password.") }}</p>
          </div>
        </article>
      </div>

      <div class="hero-meta">
        <div class="meta-item">
          <span>{{ tr("密码长度", "Password Length") }}</span>
          <strong class="mono">{{ passwordPolicy.min_length }}</strong>
        </div>
        <div class="meta-item">
          <span>{{ tr("最低强度", "Minimum Score") }}</span>
          <strong class="mono">{{ passwordPolicy.min_strength_score }}/4</strong>
        </div>
        <div class="meta-item">
          <span>{{ tr("安全校验", "Security Check") }}</span>
          <strong>{{ humanVerifyEnabled ? tr("已启用", "Enabled") : tr("未启用", "Disabled") }}</strong>
        </div>
      </div>
    </aside>

    <article class="surface surface-dashed stack auth-card">
      <header class="section-head auth-head">
        <div class="section-title">
          <p>{{ tr("账号入口", "Account Access") }}</p>
          <h2>{{ mode === "login" ? tr("登录", "Sign In") : tr("注册", "Register") }}</h2>
        </div>
        <div class="context-menu auth-mode-switch" role="tablist" :aria-label="tr('认证模式切换', 'Auth mode switch')">
          <button
            class="btn-line mode-btn"
            :class="{ active: mode === 'login' }"
            type="button"
            :aria-pressed="mode === 'login'"
            @click="switchMode('login')"
          >
            {{ tr("登录", "Sign In") }}
          </button>
          <button
            class="btn-line mode-btn"
            :class="{ active: mode === 'register' }"
            type="button"
            :aria-pressed="mode === 'register'"
            @click="switchMode('register')"
          >
            {{ tr("注册", "Register") }}
          </button>
        </div>
      </header>

      <form class="form-grid auth-form" @submit.prevent="handleAuthSubmit">
        <label v-if="mode === 'register'" class="field-block">
          <span>{{ tr("用户名", "Username") }}</span>
          <input
            v-model.trim="registerForm.username"
            required
            minlength="3"
            maxlength="32"
            autocomplete="username"
            :placeholder="tr('请输入用户名', 'Enter username')"
          />
        </label>

        <label class="field-block">
          <span>{{ mode === "login" ? tr("用户名或邮箱", "Username or email") : tr("邮箱", "Email") }}</span>
          <input
            v-model.trim="authForm.identifier"
            :type="mode === 'login' ? 'text' : 'email'"
            required
            maxlength="128"
            :autocomplete="mode === 'login' ? 'username' : 'email'"
            :placeholder="mode === 'login' ? tr('用户名或邮箱', 'Username or email') : 'name@example.com'"
          />
        </label>

        <label class="field-block">
          <span>{{ tr("密码", "Password") }}</span>
          <div class="password-row">
            <input
              v-model="authForm.password"
              :type="showPassword ? 'text' : 'password'"
              required
              :minlength="passwordPolicy.min_length"
              :autocomplete="mode === 'login' ? 'current-password' : 'new-password'"
              :placeholder="tr('请输入密码', 'Enter password')"
            />
            <button
              class="btn-line btn-compact visibility-btn"
              type="button"
              :aria-label="showPassword ? tr('隐藏密码', 'Hide password') : tr('显示密码', 'Show password')"
              @click="showPassword = !showPassword"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12Z" />
                <circle cx="12" cy="12" r="2.6" />
              </svg>
            </button>
          </div>
        </label>

        <label v-if="mode === 'register'" class="field-block">
          <span>{{ tr("确认密码", "Confirm Password") }}</span>
          <div class="password-row">
            <input
              v-model="authForm.passwordConfirm"
              :type="showPasswordConfirm ? 'text' : 'password'"
              required
              :minlength="passwordPolicy.min_length"
              autocomplete="new-password"
              :placeholder="tr('请再次输入密码', 'Repeat password')"
            />
            <button
              class="btn-line btn-compact visibility-btn"
              type="button"
              :aria-label="showPasswordConfirm ? tr('隐藏确认密码', 'Hide confirm password') : tr('显示确认密码', 'Show confirm password')"
              @click="showPasswordConfirm = !showPasswordConfirm"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12Z" />
                <circle cx="12" cy="12" r="2.6" />
              </svg>
            </button>
          </div>
        </label>

        <section v-if="mode === 'register'" class="password-lab">
          <div class="row-between">
            <h3>{{ tr("密码强度", "Password Strength") }}</h3>
            <span class="badge">{{ registerStrengthLabel }}</span>
          </div>
          <div class="strength-track">
            <span :class="['strength-fill', `level-${registerStrength.score}`]" :style="registerStrengthStyle"></span>
          </div>
          <p class="soft strength-meta">
            {{ tr(`估算破解时间：${registerCrackTimeLabel}`, `Estimated crack time: ${registerCrackTimeLabel}`) }}
          </p>
          <div class="check-grid">
            <p :class="checkClass(registerStrength.checks.length)">{{ tr(`长度至少 ${passwordPolicy.min_length} 位`, `Length >= ${passwordPolicy.min_length}`) }}</p>
            <p :class="checkClass(registerStrength.checks.lowercase)">{{ tr("包含小写字母", "Contains lowercase") }}</p>
            <p :class="checkClass(registerStrength.checks.uppercase)">{{ tr("包含大写字母", "Contains uppercase") }}</p>
            <p :class="checkClass(registerStrength.checks.digit)">{{ tr("包含数字", "Contains digit") }}</p>
            <p v-if="passwordPolicy.require_symbol" :class="checkClass(registerStrength.checks.symbol)">{{ tr("包含符号", "Contains symbol") }}</p>
            <p :class="checkClass(registerStrength.checks.unique)">{{ tr(`至少 ${passwordPolicy.min_unique_chars} 个唯一字符`, `At least ${passwordPolicy.min_unique_chars} unique chars`) }}</p>
            <p :class="checkClass(registerStrength.checks.noWeakPattern)">{{ tr("无弱口令模式", "No weak/common pattern") }}</p>
            <p :class="checkClass(registerStrength.checks.noSequence)">{{ tr("无连续字符序列", "No sequential run") }}</p>
            <p :class="checkClass(registerStrength.checks.noRepeatingRuns)">{{ tr("无重复字符序列", "No repeating run") }}</p>
            <p :class="checkClass(registerStrength.checks.noIdentityContains)">{{ tr("不包含用户名/邮箱前缀", "No username/email local-part") }}</p>
          </div>
          <p v-if="authForm.password && authForm.passwordConfirm && authForm.password !== authForm.passwordConfirm" class="warn">
            {{ tr("两次输入的密码不一致。", "Passwords do not match.") }}
          </p>
        </section>

        <section v-if="humanVerifyEnabled" class="captcha-sheet">
          <div class="row-between captcha-head">
            <h3>{{ tr("人机验证", "Human Verification") }}</h3>
            <button class="btn-line btn-compact" type="button" :disabled="!captchaReady || submitting" @click="resetCaptcha">
              {{ tr("刷新", "Reset") }}
            </button>
          </div>
          <div ref="captchaContainerRef" class="captcha-slot" :class="{ pending: captchaReady && !captchaToken }"></div>
          <p v-if="captchaError" class="warn">{{ captchaError }}</p>
        </section>

        <button
          class="btn-solid auth-submit"
          type="submit"
          :disabled="submitting || (humanVerifyEnabled && !captchaReady)"
        >
          {{
            submitting
              ? tr("处理中...", "Processing...")
              : mode === "login"
                ? tr("登录", "Sign In")
                : tr("注册", "Register")
          }}
        </button>
      </form>

      <div v-if="mode === 'login'" class="auth-foot">
        <button class="btn-link auth-link" type="button" @click="toggleRecoveryPanel">
          {{ showRecoveryPanel ? tr("收起找回密码", "Hide recovery") : tr("忘记密码？", "Forgot password?") }}
        </button>
        <button class="btn-link auth-link" type="button" @click="switchMode('register')">
          {{ tr("没有账号？去注册", "No account? Register") }}
        </button>
      </div>

      <div v-else class="auth-foot auth-foot-single">
        <button class="btn-link auth-link" type="button" @click="switchMode('login')">
          {{ tr("已有账号？去登录", "Already have an account? Sign in") }}
        </button>
      </div>

      <section v-if="mode === 'register' && pendingVerificationEmail" class="verify-sheet">
        <div class="row-between verify-head">
          <h3>{{ tr("邮箱验证", "Email Verification") }}</h3>
          <button class="btn-line btn-compact" type="button" :disabled="requestingVerifyEmail" @click="handleRequestEmailVerification">
            {{ requestingVerifyEmail ? tr("发送中...", "Sending...") : tr("重发验证邮件", "Resend") }}
          </button>
        </div>
        <p class="soft verify-target">{{ pendingVerificationEmail }}</p>
        <form class="form-grid verify-token-form" @submit.prevent="handleConfirmEmailVerification(false)">
          <label>
            <span>{{ tr("验证令牌", "Verification Token") }}</span>
            <input
              v-model.trim="verifyForm.token"
              maxlength="256"
              required
              :placeholder="tr('输入邮件中的令牌', 'Enter token from email')"
            />
          </label>
          <button class="btn-line" type="submit" :disabled="confirmingVerifyToken">
            {{ confirmingVerifyToken ? tr("确认中...", "Confirming...") : tr("确认验证", "Confirm") }}
          </button>
        </form>
      </section>

      <section v-if="showRecoveryPanel" class="reset-sheet recovery-panel">
        <header class="recovery-head">
          <h3>{{ tr("找回密码", "Password Recovery") }}</h3>
          <button class="btn-line btn-compact" type="button" @click="showRecoveryPanel = false">{{ tr("关闭", "Close") }}</button>
        </header>

        <form class="form-grid recovery-block" @submit.prevent="handleRequestPasswordReset">
          <label>
            <span>{{ tr("重置邮箱", "Reset Email") }}</span>
            <input v-model.trim="resetRequestForm.email" type="email" required maxlength="128" placeholder="name@example.com" />
          </label>
          <button class="btn-line" type="submit" :disabled="requestingResetEmail">
            {{ requestingResetEmail ? tr("发送中...", "Sending...") : tr("发送重置邮件", "Send Reset Email") }}
          </button>
        </form>

        <form class="form-grid recovery-block" @submit.prevent="handleConfirmPasswordReset">
          <label>
            <span>{{ tr("重置令牌", "Reset Token") }}</span>
            <input v-model.trim="resetConfirmForm.token" required maxlength="256" :placeholder="tr('输入邮件中的令牌', 'Enter token from email')" />
          </label>

          <label>
            <span>{{ tr("新密码", "New Password") }}</span>
            <div class="password-row">
              <input
                v-model="resetConfirmForm.password"
                :type="showResetPassword ? 'text' : 'password'"
                required
                :minlength="passwordPolicy.min_length"
                autocomplete="new-password"
              />
              <button class="btn-line btn-compact visibility-btn" type="button" @click="showResetPassword = !showResetPassword">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12Z" />
                  <circle cx="12" cy="12" r="2.6" />
                </svg>
              </button>
            </div>
          </label>

          <label>
            <span>{{ tr("确认新密码", "Confirm New Password") }}</span>
            <div class="password-row">
              <input
                v-model="resetConfirmForm.passwordConfirm"
                :type="showResetPasswordConfirm ? 'text' : 'password'"
                required
                :minlength="passwordPolicy.min_length"
                autocomplete="new-password"
              />
              <button class="btn-line btn-compact visibility-btn" type="button" @click="showResetPasswordConfirm = !showResetPasswordConfirm">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12Z" />
                  <circle cx="12" cy="12" r="2.6" />
                </svg>
              </button>
            </div>
          </label>

          <section class="password-lab compact">
            <div class="row-between">
              <h3>{{ tr("新密码强度", "New Password Strength") }}</h3>
              <span class="badge">{{ resetStrengthLabel }}</span>
            </div>
            <div class="strength-track">
              <span :class="['strength-fill', `level-${resetStrength.score}`]" :style="resetStrengthStyle"></span>
            </div>
            <p class="soft strength-meta">
              {{ tr(`估算破解时间：${resetCrackTimeLabel}`, `Estimated crack time: ${resetCrackTimeLabel}`) }}
            </p>
            <p v-if="resetConfirmForm.password && resetConfirmForm.passwordConfirm && resetConfirmForm.password !== resetConfirmForm.passwordConfirm" class="warn">
              {{ tr("两次输入的密码不一致。", "Passwords do not match.") }}
            </p>
          </section>

          <button class="btn-solid" type="submit" :disabled="confirmingResetPassword">
            {{ confirmingResetPassword ? tr("提交中...", "Submitting...") : tr("确认重置密码", "Confirm Password Reset") }}
          </button>
        </form>
      </section>

      <p v-if="message" class="message auth-feedback">{{ message }}</p>
      <p v-if="error" class="error auth-feedback">{{ error }}</p>
    </article>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import {
  ApiClientError,
  confirmEmailVerification,
  confirmPasswordReset,
  getPasswordPolicy,
  requestEmailVerification,
  requestPasswordReset,
  type PasswordPolicySnapshot
} from "../api/client";
import {
  DEFAULT_PASSWORD_POLICY,
  evaluatePasswordStrength,
  formatCrackTime,
  type PasswordStrengthReport
} from "../composables/usePasswordStrength";
import { useL10n } from "../composables/useL10n";
import { useAuthStore } from "../stores/auth";
import { useUiStore } from "../stores/ui";

type AuthMode = "login" | "register";

type TurnstileRenderOptions = {
  sitekey: string;
  theme?: "auto" | "light" | "dark";
  callback?: (token: string) => void;
  "expired-callback"?: () => void;
  "error-callback"?: () => void;
};

type TurnstileApi = {
  render: (container: string | HTMLElement, options: TurnstileRenderOptions) => string | number;
  reset: (widgetId?: string | number) => void;
  remove: (widgetId: string | number) => void;
};

declare global {
  interface Window {
    turnstile?: TurnstileApi;
  }
}

const TURNSTILE_SCRIPT_ID = "cf-turnstile-api";
const TURNSTILE_SCRIPT_SRC = "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";

const authStore = useAuthStore();
const uiStore = useUiStore();
const { tr } = useL10n();
const router = useRouter();
const route = useRoute();

const mode = ref<AuthMode>("login");
const showRecoveryPanel = ref(false);
const showPassword = ref(false);
const showPasswordConfirm = ref(false);
const showResetPassword = ref(false);
const showResetPasswordConfirm = ref(false);

const authForm = reactive({
  identifier: "",
  password: "",
  passwordConfirm: ""
});

const registerForm = reactive({
  username: ""
});

const verifyForm = reactive({
  token: ""
});

const resetRequestForm = reactive({
  email: ""
});

const resetConfirmForm = reactive({
  token: "",
  password: "",
  passwordConfirm: ""
});

const submitting = ref(false);
const requestingVerifyEmail = ref(false);
const confirmingVerifyToken = ref(false);
const requestingResetEmail = ref(false);
const confirmingResetPassword = ref(false);
const message = ref("");
const error = ref("");

const pendingVerificationEmail = ref("");

const captchaContainerRef = ref<HTMLElement | null>(null);
const captchaToken = ref("");
const captchaError = ref("");
const captchaReady = ref(false);
const captchaWidgetId = ref<string | number | null>(null);

const passwordPolicy = ref<PasswordPolicySnapshot>(DEFAULT_PASSWORD_POLICY);
const handledVerifyToken = ref("");

const turnstileSiteKey = ((import.meta.env.VITE_TURNSTILE_SITE_KEY as string | undefined) ?? "").trim();
const humanVerifyEnabled = computed(() => turnstileSiteKey.length > 0);

const redirectPath = computed(() => {
  const raw = route.query.redirect;
  if (typeof raw === "string" && raw.startsWith("/")) {
    return raw;
  }
  return "/contests";
});

const registerStrength = computed(() =>
  evaluatePasswordStrength({
    password: authForm.password,
    policy: passwordPolicy.value,
    username: registerForm.username,
    email: authForm.identifier
  })
);

const resetStrength = computed(() =>
  evaluatePasswordStrength({
    password: resetConfirmForm.password,
    policy: passwordPolicy.value,
    email: resetRequestForm.email
  })
);

const registerStrengthLabel = computed(() => strengthLabel(registerStrength.value.score));
const resetStrengthLabel = computed(() => strengthLabel(resetStrength.value.score));
const registerCrackTimeLabel = computed(() => formatCrackTime(registerStrength.value.crackTimeSeconds));
const resetCrackTimeLabel = computed(() => formatCrackTime(resetStrength.value.crackTimeSeconds));

const registerStrengthStyle = computed(() => ({
  width: `${(registerStrength.value.score / 4) * 100}%`
}));

const resetStrengthStyle = computed(() => ({
  width: `${(resetStrength.value.score / 4) * 100}%`
}));

watch(mode, () => {
  resetFeedback();
  showPassword.value = false;
  showPasswordConfirm.value = false;
  if (mode.value !== "login") {
    showRecoveryPanel.value = false;
  }
  if (humanVerifyEnabled.value) {
    resetCaptcha();
  }
});

watch(
  () => route.query,
  () => {
    applyRouteTokenHints();
  },
  { deep: true, immediate: true }
);

function resetFeedback() {
  message.value = "";
  error.value = "";
}

function switchMode(nextMode: AuthMode) {
  mode.value = nextMode;
}

function toggleRecoveryPanel() {
  showRecoveryPanel.value = !showRecoveryPanel.value;
  if (showRecoveryPanel.value && !resetRequestForm.email && authForm.identifier.includes("@")) {
    resetRequestForm.email = authForm.identifier.trim();
  }
}

function checkClass(ok: boolean) {
  return ok ? "check-ok" : "check-fail";
}

function strengthLabel(score: number) {
  if (score <= 0) {
    return tr("很弱", "Very weak");
  }
  if (score === 1) {
    return tr("较弱", "Weak");
  }
  if (score === 2) {
    return tr("中等", "Fair");
  }
  if (score === 3) {
    return tr("较强", "Strong");
  }
  return tr("很强", "Very strong");
}

function passwordAcceptable(report: PasswordStrengthReport): boolean {
  const checks = Object.values(report.checks);
  return checks.every(Boolean) && report.score >= passwordPolicy.value.min_strength_score;
}

function resetCaptcha() {
  captchaToken.value = "";
  captchaError.value = "";

  if (window.turnstile && captchaWidgetId.value !== null) {
    window.turnstile.reset(captchaWidgetId.value);
  }
}

function applyRouteTokenHints() {
  const resetToken = typeof route.query.reset_token === "string" ? route.query.reset_token.trim() : "";
  if (resetToken) {
    mode.value = "login";
    showRecoveryPanel.value = true;
    if (!resetConfirmForm.token) {
      resetConfirmForm.token = resetToken;
    }
  }

  const verifyToken = typeof route.query.verify_token === "string" ? route.query.verify_token.trim() : "";
  if (verifyToken && verifyToken !== handledVerifyToken.value) {
    handledVerifyToken.value = verifyToken;
    mode.value = "register";
    verifyForm.token = verifyToken;
    void handleConfirmEmailVerification(true);
  }
}

async function clearQueryToken(name: string) {
  if (!(name in route.query)) {
    return;
  }

  const nextQuery = { ...route.query };
  delete nextQuery[name];
  await router.replace({ path: route.path, query: nextQuery });
}

async function loadPasswordPolicy() {
  try {
    const response = await getPasswordPolicy();
    passwordPolicy.value = response.policy;
  } catch {
    passwordPolicy.value = DEFAULT_PASSWORD_POLICY;
  }
}

async function ensureTurnstileScriptLoaded() {
  if (!humanVerifyEnabled.value) {
    return;
  }

  if (window.turnstile) {
    captchaReady.value = true;
    return;
  }

  let script = document.getElementById(TURNSTILE_SCRIPT_ID) as HTMLScriptElement | null;
  if (!script) {
    script = document.createElement("script");
    script.id = TURNSTILE_SCRIPT_ID;
    script.src = TURNSTILE_SCRIPT_SRC;
    script.async = true;
    script.defer = true;
    document.head.appendChild(script);
  }

  await new Promise<void>((resolve, reject) => {
    if (window.turnstile) {
      resolve();
      return;
    }

    const target = script as HTMLScriptElement;
    const timeoutId = window.setTimeout(() => {
      target.removeEventListener("load", onLoad);
      target.removeEventListener("error", onError);
      reject(new Error("turnstile script timeout"));
    }, 10_000);

    const onLoad = () => {
      window.clearTimeout(timeoutId);
      target.removeEventListener("error", onError);
      if (!window.turnstile) {
        reject(new Error("turnstile not available"));
        return;
      }
      resolve();
    };

    const onError = () => {
      window.clearTimeout(timeoutId);
      target.removeEventListener("load", onLoad);
      reject(new Error("turnstile script error"));
    };

    target.addEventListener("load", onLoad, { once: true });
    target.addEventListener("error", onError, { once: true });
  });

  captchaReady.value = true;
}

function renderCaptchaWidget() {
  if (!humanVerifyEnabled.value || !window.turnstile || !captchaContainerRef.value) {
    return;
  }

  if (captchaWidgetId.value !== null) {
    try {
      window.turnstile.remove(captchaWidgetId.value);
    } catch {
      // no-op
    }
    captchaWidgetId.value = null;
  }

  captchaContainerRef.value.innerHTML = "";
  captchaToken.value = "";
  captchaError.value = "";

  captchaWidgetId.value = window.turnstile.render(captchaContainerRef.value, {
    sitekey: turnstileSiteKey,
    theme: "auto",
    callback: (token: string) => {
      captchaToken.value = token;
      captchaError.value = "";
    },
    "expired-callback": () => {
      captchaToken.value = "";
      captchaError.value = tr("验证码已过期，请重新验证。", "Verification expired. Please verify again.");
    },
    "error-callback": () => {
      captchaToken.value = "";
      captchaError.value = tr("验证码加载失败，请稍后重试。", "Verification failed to load. Please try again.");
    }
  });
}

onMounted(async () => {
  await loadPasswordPolicy();

  if (!humanVerifyEnabled.value) {
    return;
  }

  try {
    await ensureTurnstileScriptLoaded();
    await nextTick();
    renderCaptchaWidget();
  } catch {
    captchaReady.value = false;
    captchaError.value = tr("人机验证脚本加载失败。", "Failed to load human verification script.");
  }
});

onUnmounted(() => {
  if (window.turnstile && captchaWidgetId.value !== null) {
    try {
      window.turnstile.remove(captchaWidgetId.value);
    } catch {
      // no-op
    }
  }
});

async function handleAuthSubmit() {
  resetFeedback();

  const identifier = authForm.identifier.trim();
  if (!identifier) {
    error.value = tr("请输入账号标识。", "Please enter account identifier.");
    return;
  }

  if (humanVerifyEnabled.value && !captchaToken.value) {
    captchaError.value = tr("请先完成人机验证。", "Please complete human verification first.");
    return;
  }

  submitting.value = true;

  try {
    if (mode.value === "login") {
      await authStore.loginWithPassword({
        identifier,
        password: authForm.password,
        captcha_token: humanVerifyEnabled.value ? captchaToken.value : undefined
      });
      message.value = tr("登录成功，正在跳转...", "Signed in, redirecting...");
      uiStore.success(tr("登录成功", "Sign in succeeded"), tr("欢迎回来。", "Welcome back."), 2200);
      await router.replace(redirectPath.value);
      return;
    }

    const username = registerForm.username.trim();
    if (!username) {
      error.value = tr("请输入用户名。", "Please enter username.");
      return;
    }

    if (authForm.password !== authForm.passwordConfirm) {
      error.value = tr("两次输入的密码不一致。", "Passwords do not match.");
      return;
    }

    if (!passwordAcceptable(registerStrength.value)) {
      error.value = tr("密码强度不足，请按规则提升密码强度。", "Password is too weak. Please satisfy policy checks.");
      return;
    }

    const response = await authStore.registerWithPassword({
      username,
      email: identifier,
      password: authForm.password,
      password_confirm: authForm.passwordConfirm,
      captcha_token: humanVerifyEnabled.value ? captchaToken.value : undefined
    });

    message.value = response.message;

    if (response.auth) {
      uiStore.success(
        tr("注册成功", "Registration succeeded"),
        tr("账号已创建并完成登录。", "Account created and signed in."),
        2200
      );
      await router.replace(redirectPath.value);
      return;
    }

    pendingVerificationEmail.value = identifier;
    authForm.password = "";
    authForm.passwordConfirm = "";

    uiStore.info(
      tr("注册成功", "Registration succeeded"),
      tr("请完成邮箱验证后再登录。", "Complete email verification before signing in.")
    );

    if (humanVerifyEnabled.value) {
      resetCaptcha();
    }
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("请求失败", "Request failed");
    uiStore.error(tr("认证失败", "Authentication failed"), error.value);
    uiStore.alertError(tr("认证错误", "Authentication error"), error.value);
    if (humanVerifyEnabled.value) {
      resetCaptcha();
    }
  } finally {
    submitting.value = false;
  }
}

async function handleRequestEmailVerification() {
  resetFeedback();

  const email = (pendingVerificationEmail.value || authForm.identifier.trim()).trim();
  if (!email) {
    error.value = tr("请先填写注册邮箱。", "Please provide registration email first.");
    return;
  }

  requestingVerifyEmail.value = true;
  try {
    const response = await requestEmailVerification({ email });
    message.value = response.message;
    uiStore.info(tr("验证邮件", "Verification Email"), response.message, 2600);
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("请求失败", "Request failed");
    uiStore.error(tr("发送失败", "Failed to send"), error.value);
  } finally {
    requestingVerifyEmail.value = false;
  }
}

async function handleConfirmEmailVerification(fromRouteToken: boolean) {
  resetFeedback();

  const token = verifyForm.token.trim();
  if (!token) {
    if (!fromRouteToken) {
      error.value = tr("请输入验证令牌。", "Please enter verification token.");
    }
    return;
  }

  confirmingVerifyToken.value = true;
  try {
    const response = await confirmEmailVerification({ token });
    message.value = response.message;
    uiStore.success(tr("邮箱验证成功", "Email verified"), response.message, 2600);
    verifyForm.token = "";
    pendingVerificationEmail.value = "";
    mode.value = "login";
    await clearQueryToken("verify_token");
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("请求失败", "Request failed");
    uiStore.error(tr("验证失败", "Verification failed"), error.value);
  } finally {
    confirmingVerifyToken.value = false;
  }
}

async function handleRequestPasswordReset() {
  resetFeedback();

  const email = resetRequestForm.email.trim();
  if (!email) {
    error.value = tr("请输入邮箱。", "Please enter email.");
    return;
  }

  requestingResetEmail.value = true;
  try {
    const response = await requestPasswordReset({ email });
    message.value = response.message;
    uiStore.info(tr("重置邮件", "Reset Email"), response.message, 2600);
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("请求失败", "Request failed");
    uiStore.error(tr("发送失败", "Failed to send"), error.value);
  } finally {
    requestingResetEmail.value = false;
  }
}

async function handleConfirmPasswordReset() {
  resetFeedback();

  const token = resetConfirmForm.token.trim();
  if (!token) {
    error.value = tr("请输入重置令牌。", "Please enter reset token.");
    return;
  }

  if (resetConfirmForm.password !== resetConfirmForm.passwordConfirm) {
    error.value = tr("两次输入的密码不一致。", "Passwords do not match.");
    return;
  }

  if (!passwordAcceptable(resetStrength.value)) {
    error.value = tr("新密码强度不足，请按规则提升密码强度。", "New password is too weak. Please satisfy policy checks.");
    return;
  }

  confirmingResetPassword.value = true;
  try {
    const response = await confirmPasswordReset({
      token,
      new_password: resetConfirmForm.password,
      new_password_confirm: resetConfirmForm.passwordConfirm
    });

    message.value = response.message;
    uiStore.success(tr("密码重置成功", "Password reset succeeded"), response.message, 2800);

    authForm.password = "";
    authForm.passwordConfirm = "";
    resetConfirmForm.password = "";
    resetConfirmForm.passwordConfirm = "";
    showRecoveryPanel.value = false;
    mode.value = "login";
    await clearQueryToken("reset_token");
  } catch (err) {
    error.value = err instanceof ApiClientError ? err.message : tr("请求失败", "Request failed");
    uiStore.error(tr("重置失败", "Reset failed"), error.value);
  } finally {
    confirmingResetPassword.value = false;
  }
}
</script>

<style scoped>
.login-shell {
  min-height: min(760px, calc(100vh - 156px));
  display: grid;
  gap: 0.92rem;
  grid-template-columns: minmax(0, 0.95fr) minmax(0, 1.05fr);
  align-items: stretch;
}

.auth-hero,
.auth-card {
  min-height: 100%;
}

.auth-hero {
  position: relative;
  overflow: clip;
}

.auth-hero::after {
  content: "";
  position: absolute;
  width: 380px;
  height: 380px;
  right: -140px;
  top: -150px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.46) 0%, transparent 72%);
  pointer-events: none;
}

.hero-head {
  display: flex;
  align-items: center;
  gap: 0.62rem;
  position: relative;
  z-index: 1;
}

.hero-mark {
  width: 2.9rem;
  height: 2.9rem;
  border-radius: 999px;
  display: grid;
  place-items: center;
  background: var(--glass-strong);
  box-shadow:
    inset 0 0 0 1px var(--line-mid),
    inset 0 0 0 8px color-mix(in srgb, var(--glass-strong) 72%, transparent 28%);
}

.hero-mark span {
  font-size: 0.74rem;
}

.hero-heading {
  display: grid;
  gap: 0.16rem;
}

.hero-heading h1 {
  font-size: clamp(1.24rem, 2.1vw, 1.84rem);
}

.hero-intro {
  font-size: 0.92rem;
  line-height: 1.62;
  max-width: 62ch;
}

.showcase-eyebrow {
  font-size: 0.73rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--fg-2);
}

.hero-points {
  display: grid;
  gap: 0.56rem;
  position: relative;
  z-index: 1;
}

.hero-point {
  display: grid;
  grid-template-columns: 1.2rem minmax(0, 1fr);
  gap: 0.52rem;
  align-items: start;
  border-radius: var(--radius-md);
  padding: 0.66rem 0.72rem;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
}

.hero-point svg {
  width: 1.14rem;
  height: 1.14rem;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.66;
  stroke-linecap: round;
  stroke-linejoin: round;
  margin-top: 0.1rem;
}

.hero-point h3 {
  font-size: 0.94rem;
  margin-bottom: 0.08rem;
}

.hero-point p {
  font-size: 0.84rem;
}

.hero-meta {
  display: grid;
  gap: 0.52rem;
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.meta-item {
  border-radius: var(--radius-md);
  padding: 0.58rem 0.66rem;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  display: grid;
  gap: 0.16rem;
}

.meta-item span {
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--fg-2);
}

.meta-item strong {
  font-size: 0.92rem;
}

.auth-head {
  align-items: center;
}

.auth-mode-switch {
  gap: 0.2rem;
}

.mode-btn.active {
  background: var(--fg-0);
  color: var(--bg-0);
}

.auth-form {
  gap: 0.68rem;
}

.field-block {
  display: grid;
  gap: 0.3rem;
}

.field-block span {
  font-size: 0.8rem;
  color: var(--fg-2);
}

.password-row {
  position: relative;
  display: flex;
  align-items: center;
}

.password-row input {
  padding-right: 3.1rem;
}

.visibility-btn {
  position: absolute;
  right: 0.3rem;
  top: 50%;
  transform: translateY(-50%);
  height: 1.86rem;
  min-height: 1.86rem;
  width: 2.38rem;
  padding: 0;
}

.visibility-btn svg {
  width: 1rem;
  height: 1rem;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.66;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.visibility-btn:hover,
.visibility-btn:focus-visible {
  transform: translateY(-50%);
}

.password-lab {
  border-radius: var(--radius-md);
  padding: 0.64rem 0.72rem;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  display: grid;
  gap: 0.5rem;
}

.password-lab h3 {
  font-size: 0.92rem;
}

.password-lab.compact {
  padding-top: 0.6rem;
}

.strength-track {
  height: 6px;
  border-radius: 999px;
  background: var(--glass-soft);
  overflow: hidden;
}

.strength-fill {
  display: block;
  height: 100%;
  transition: width 220ms ease;
}

.strength-fill.level-0 {
  background: color-mix(in srgb, var(--danger) 72%, transparent 28%);
}

.strength-fill.level-1 {
  background: color-mix(in srgb, #936329 78%, transparent 22%);
}

.strength-fill.level-2 {
  background: color-mix(in srgb, #8a7b2c 78%, transparent 22%);
}

.strength-fill.level-3 {
  background: color-mix(in srgb, #2b7e57 78%, transparent 22%);
}

.strength-fill.level-4 {
  background: color-mix(in srgb, #1a6d56 86%, transparent 14%);
}

.strength-meta {
  font-size: 0.78rem;
}

.check-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.28rem 0.52rem;
}

.check-grid p {
  font-size: 0.78rem;
}

.check-ok {
  color: var(--ok);
}

.check-fail {
  color: var(--fg-2);
}

.captcha-sheet {
  border-radius: var(--radius-md);
  padding: 0.66rem 0.72rem;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  display: grid;
  gap: 0.52rem;
}

.captcha-sheet h3 {
  font-size: 0.9rem;
}

.captcha-head {
  align-items: center;
}

.captcha-slot {
  position: relative;
  min-height: 82px;
  display: grid;
  align-items: center;
  justify-items: start;
  border-radius: var(--radius-sm);
  padding: 0.24rem;
  background: var(--glass-soft);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  overflow: hidden;
}

.captcha-slot.pending::after {
  content: "";
  position: absolute;
  left: 0.24rem;
  right: 0.24rem;
  bottom: 0.24rem;
  height: 1px;
  background: repeating-linear-gradient(90deg, transparent 0 6px, var(--line-mid) 6px 11px);
  pointer-events: none;
}

.captcha-slot:empty::before {
  content: "...";
  color: var(--fg-2);
}

.captcha-slot :deep(.cf-turnstile) {
  width: 100%;
}

.captcha-slot :deep(iframe) {
  max-width: 100%;
}

.auth-submit {
  margin-top: 0.12rem;
  min-height: 2.34rem;
  font-size: 1rem;
  letter-spacing: 0.04em;
  width: 100%;
}

.auth-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.48rem;
}

.auth-foot-single {
  justify-content: flex-end;
}

.auth-link {
  padding: 0;
  min-height: auto;
  border-radius: 0;
  background: transparent;
  color: var(--fg-2);
}

.auth-link:hover {
  color: var(--fg-0);
  transform: none;
}

.verify-sheet {
  border-radius: var(--radius-md);
  padding: 0.66rem 0.72rem;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  display: grid;
  gap: 0.5rem;
}

.verify-head h3 {
  font-size: 0.92rem;
}

.verify-target {
  font-size: 0.84rem;
}

.verify-token-form {
  gap: 0.44rem;
}

.reset-sheet {
  border-radius: var(--radius-md);
  padding: 0.66rem 0.72rem;
  background: var(--glass-mid);
  box-shadow: inset 0 -1px 0 var(--line-soft);
  display: grid;
  gap: 0.52rem;
}

.recovery-panel {
  gap: 0.68rem;
}

.recovery-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.recovery-head h3 {
  font-size: 0.94rem;
}

.recovery-block {
  padding-top: 0.4rem;
  border-top: 1px dashed var(--line-mid);
}

.auth-feedback {
  padding-inline: 0.06rem;
  font-size: 0.9rem;
}

@media (max-width: 1180px) {
  .check-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 1080px) {
  .login-shell {
    grid-template-columns: 1fr;
    min-height: auto;
  }

  .hero-meta {
    grid-template-columns: 1fr;
  }
}
</style>
