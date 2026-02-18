const api = process.env.API_BASE || "http://127.0.0.1:8080/api/v1";
const adminUser = process.env.ADMIN_USER || "admin";
const adminPassword = process.env.ADMIN_PASSWORD || "admin123456";
const userPassword = process.env.USER_PASSWORD || "password123";

async function request(path, { method = "GET", token, body } = {}) {
  const response = await fetch(`${api}${path}`, {
    method,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });

  const text = await response.text();
  let payload;
  try {
    payload = text ? JSON.parse(text) : null;
  } catch {
    payload = text;
  }

  if (!response.ok) {
    throw new Error(`${method} ${path} failed (${response.status}): ${text}`);
  }

  return payload;
}

function isoOffset(minutes) {
  return new Date(Date.now() + minutes * 60_000).toISOString();
}

function shortSuffix() {
  const now = Date.now().toString().slice(-6);
  const rnd = Math.floor(Math.random() * 1000)
    .toString()
    .padStart(3, "0");
  return `${now}${rnd}`;
}

function toWsUrl(httpApiUrl, contestId, token) {
  const base = httpApiUrl.replace(/^http:/, "ws:").replace(/^https:/, "wss:");
  return `${base}/contests/${contestId}/scoreboard/ws?access_token=${encodeURIComponent(token)}`;
}

async function registerUser(prefix, suffix) {
  const username = `${prefix}${suffix}`;
  const register = await request("/auth/register", {
    method: "POST",
    body: {
      username,
      email: `${username}@example.com`,
      password: userPassword,
    },
  });

  await request("/teams", {
    method: "POST",
    token: register.access_token,
    body: {
      name: `${prefix}-team-${suffix}`.slice(0, 32),
    },
  });

  return register.access_token;
}

async function run() {
  if (typeof WebSocket !== "function") {
    throw new Error("global WebSocket is unavailable in current Node runtime");
  }

  const suffix = shortSuffix();

  const adminLogin = await request("/auth/login", {
    method: "POST",
    body: {
      identifier: adminUser,
      password: adminPassword,
    },
  });
  const adminToken = adminLogin.access_token;

  const contest = await request("/admin/contests", {
    method: "POST",
    token: adminToken,
    body: {
      title: `WS Smoke ${suffix}`,
      slug: `ws-smoke-${suffix}`,
      visibility: "public",
      status: "running",
      start_at: isoOffset(-1),
      end_at: isoOffset(120),
    },
  });

  const challenge = await request("/admin/challenges", {
    method: "POST",
    token: adminToken,
    body: {
      title: `WS Challenge ${suffix}`,
      slug: `ws-chal-${suffix}`,
      category: "web",
      difficulty: "easy",
      challenge_type: "static",
      flag_mode: "static",
      flag_hash: "ctf{ws-smoke-flag}",
      status: "published",
      is_visible: true,
      static_score: 100,
      min_score: 100,
      max_score: 100,
    },
  });

  await request(`/admin/contests/${contest.id}/challenges`, {
    method: "POST",
    token: adminToken,
    body: {
      challenge_id: challenge.id,
      sort_order: 1,
    },
  });

  const watcherToken = await registerUser("wa", suffix);
  const submitterToken = await registerUser("su", suffix);

  const wsUrl = toWsUrl(api, contest.id, watcherToken);
  const wsResult = await new Promise((resolve, reject) => {
    const ws = new WebSocket(wsUrl);
    const timeout = setTimeout(() => {
      try {
        ws.close();
      } catch {}
      reject(new Error("scoreboard websocket timeout waiting for update"));
    }, 15_000);

    let sawInitialSnapshot = false;
    let sawScoreUpdate = false;

    ws.onopen = async () => {
      try {
        await request("/submissions", {
          method: "POST",
          token: submitterToken,
          body: {
            contest_id: contest.id,
            challenge_id: challenge.id,
            flag: "ctf{ws-smoke-flag}",
          },
        });
      } catch (error) {
        clearTimeout(timeout);
        reject(error);
      }
    };

    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(String(event.data));
        if (payload.event !== "scoreboard_update") {
          return;
        }

        if (!sawInitialSnapshot) {
          sawInitialSnapshot = true;
          return;
        }

        const hasScoredEntry =
          Array.isArray(payload.entries) &&
          payload.entries.some((item) => Number(item.score) >= 100);
        if (hasScoredEntry) {
          sawScoreUpdate = true;
          clearTimeout(timeout);
          ws.close();
          resolve({
            saw_initial_snapshot: sawInitialSnapshot,
            saw_score_update: sawScoreUpdate,
            entry_count: payload.entries.length,
          });
        }
      } catch (error) {
        clearTimeout(timeout);
        reject(error);
      }
    };

    ws.onerror = () => {
      clearTimeout(timeout);
      reject(new Error("scoreboard websocket connection failed"));
    };
  });

  const output = {
    contest_id: contest.id,
    challenge_id: challenge.id,
    ...wsResult,
  };
  console.log(JSON.stringify(output, null, 2));
}

run().catch((error) => {
  console.error(`[scoreboard-ws-smoke] ${error.message}`);
  process.exit(1);
});
