import type { PasswordPolicySnapshot } from "../api/client";

const COMMON_WEAK_PATTERNS = [
  "password",
  "passw0rd",
  "qwerty",
  "qwertyui",
  "qwerty123",
  "abc123",
  "letmein",
  "admin",
  "welcome",
  "iloveyou",
  "111111",
  "123456",
  "12345678",
  "123456789",
  "123123"
] as const;

export const DEFAULT_PASSWORD_POLICY: PasswordPolicySnapshot = {
  min_length: 10,
  min_strength_score: 3,
  require_lowercase: true,
  require_uppercase: true,
  require_digit: true,
  require_symbol: false,
  min_unique_chars: 6,
  block_weak_patterns: true
};

export type PasswordStrengthReport = {
  score: number;
  entropyBits: number;
  crackTimeSeconds: number;
  checks: {
    length: boolean;
    lowercase: boolean;
    uppercase: boolean;
    digit: boolean;
    symbol: boolean;
    unique: boolean;
    noWhitespace: boolean;
    noWeakPattern: boolean;
    noSequence: boolean;
    noRepeatingRuns: boolean;
    noIdentityContains: boolean;
  };
};

type EvaluatePasswordOptions = {
  password: string;
  policy?: PasswordPolicySnapshot;
  username?: string;
  email?: string;
};

export function evaluatePasswordStrength(options: EvaluatePasswordOptions): PasswordStrengthReport {
  const password = options.password ?? "";
  const policy = options.policy ?? DEFAULT_PASSWORD_POLICY;
  const chars = [...password];
  const length = chars.length;
  const uniqueChars = new Set(chars).size;

  const hasLowercase = chars.some((ch) => /[a-z]/.test(ch));
  const hasUppercase = chars.some((ch) => /[A-Z]/.test(ch));
  const hasDigit = chars.some((ch) => /[0-9]/.test(ch));
  const hasSymbol = chars.some((ch) => /[^A-Za-z0-9\s]/.test(ch));
  const hasWhitespace = chars.some((ch) => /\s/.test(ch));
  const weakPattern = containsWeakPattern(password);
  const sequence = containsSequence(password);
  const repeatingRuns = containsRepeatingRuns(password);
  const identityContains = containsIdentity(password, options.username, options.email);

  let score = 0;
  if (length >= 8) {
    score += 1;
  }
  if (length >= 12) {
    score += 1;
  }
  if (length >= 16) {
    score += 1;
  }
  if (length >= 20) {
    score += 1;
  }

  const classCount = [hasLowercase, hasUppercase, hasDigit, hasSymbol].filter(Boolean).length;
  if (classCount >= 2) {
    score += 1;
  }
  if (classCount >= 3) {
    score += 1;
  }
  if (classCount >= 4) {
    score += 1;
  }

  if (length > 0 && uniqueChars / length >= 0.55) {
    score += 1;
  }

  if (hasWhitespace) {
    score -= 1;
  }
  if (weakPattern) {
    score -= 2;
  }
  if (sequence) {
    score -= 1;
  }
  if (repeatingRuns) {
    score -= 1;
  }
  if (identityContains) {
    score -= 2;
  }

  const boundedScore = clamp(score, 0, 4);
  const charsetSize = estimateCharsetSize({
    hasLowercase,
    hasUppercase,
    hasDigit,
    hasSymbol
  });
  let entropyBits = length > 0 && charsetSize > 1 ? length * Math.log2(charsetSize) : 0;
  if (weakPattern) {
    entropyBits -= 10;
  }
  if (sequence) {
    entropyBits -= 7;
  }
  if (repeatingRuns) {
    entropyBits -= 4;
  }
  if (identityContains) {
    entropyBits -= 8;
  }
  entropyBits = Math.max(entropyBits, 0);

  const cappedEntropy = Math.min(entropyBits, 80);
  const crackTimeSeconds = Math.pow(2, cappedEntropy) / 1e10;

  return {
    score: boundedScore,
    entropyBits,
    crackTimeSeconds,
    checks: {
      length: length >= policy.min_length,
      lowercase: !policy.require_lowercase || hasLowercase,
      uppercase: !policy.require_uppercase || hasUppercase,
      digit: !policy.require_digit || hasDigit,
      symbol: !policy.require_symbol || hasSymbol,
      unique: uniqueChars >= policy.min_unique_chars,
      noWhitespace: !hasWhitespace,
      noWeakPattern: !policy.block_weak_patterns || !weakPattern,
      noSequence: !policy.block_weak_patterns || !sequence,
      noRepeatingRuns: !policy.block_weak_patterns || !repeatingRuns,
      noIdentityContains: !policy.block_weak_patterns || !identityContains
    }
  };
}

export function formatCrackTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "< 1s";
  }
  if (seconds < 1) {
    return "< 1s";
  }

  const units = [
    { label: "y", value: 365 * 24 * 3600 },
    { label: "d", value: 24 * 3600 },
    { label: "h", value: 3600 },
    { label: "m", value: 60 },
    { label: "s", value: 1 }
  ] as const;

  for (const unit of units) {
    if (seconds >= unit.value) {
      const amount = Math.floor(seconds / unit.value);
      return `${amount}${unit.label}`;
    }
  }

  return "< 1s";
}

function containsWeakPattern(password: string): boolean {
  if (!password) {
    return false;
  }

  const lowered = password.toLowerCase();
  if (COMMON_WEAK_PATTERNS.some((pattern) => lowered.includes(pattern))) {
    return true;
  }

  return [...password].every((ch) => ch === password[0]);
}

function containsRepeatingRuns(password: string): boolean {
  if (password.length < 3) {
    return false;
  }

  let run = 1;
  for (let idx = 1; idx < password.length; idx += 1) {
    if (password[idx] === password[idx - 1]) {
      run += 1;
      if (run >= 3) {
        return true;
      }
    } else {
      run = 1;
    }
  }

  return false;
}

function containsSequence(password: string): boolean {
  const lowered = password.toLowerCase();
  if (lowered.length < 4) {
    return false;
  }

  let asc = 1;
  let desc = 1;
  for (let idx = 1; idx < lowered.length; idx += 1) {
    const prev = lowered.charCodeAt(idx - 1);
    const current = lowered.charCodeAt(idx);
    const isAsciiNum = (code: number) =>
      (code >= 48 && code <= 57) || (code >= 97 && code <= 122);

    if (!isAsciiNum(prev) || !isAsciiNum(current)) {
      asc = 1;
      desc = 1;
      continue;
    }

    asc = current === prev + 1 ? asc + 1 : 1;
    desc = current + 1 === prev ? desc + 1 : 1;
    if (asc >= 4 || desc >= 4) {
      return true;
    }
  }

  return false;
}

function containsIdentity(password: string, username?: string, email?: string): boolean {
  const lowered = password.toLowerCase();
  const normalizedUsername = (username ?? "").trim().toLowerCase();
  if (normalizedUsername.length >= 3 && lowered.includes(normalizedUsername)) {
    return true;
  }

  const localPart = (email ?? "")
    .trim()
    .toLowerCase()
    .split("@")[0] ?? "";
  if (localPart.length >= 3 && lowered.includes(localPart)) {
    return true;
  }

  return false;
}

function estimateCharsetSize(input: {
  hasLowercase: boolean;
  hasUppercase: boolean;
  hasDigit: boolean;
  hasSymbol: boolean;
}): number {
  let size = 0;
  if (input.hasLowercase) {
    size += 26;
  }
  if (input.hasUppercase) {
    size += 26;
  }
  if (input.hasDigit) {
    size += 10;
  }
  if (input.hasSymbol) {
    size += 33;
  }
  return size;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
