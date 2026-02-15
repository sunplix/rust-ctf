# Init Next Steps

## Backend

1. Core migration is ready (users, teams, contests, challenges, submissions, instances).
2. DB pool and Redis manager initialization is ready in `AppState`.
3. Basic auth endpoints are ready: register, login, refresh, me.
4. Bearer auth extractor (route guard) is ready for protected APIs.
5. Basic contest/challenge list APIs and submission flow are ready.
6. Submission rate limiting and scoreboard API are ready.
7. Dynamic flag verifier and hashed static flag check are ready.
8. Script-based flag verifier is ready (metadata-driven command + timeout).
9. Scoreboard websocket push skeleton is ready (Redis pubsub trigger + ws route).
10. Instance lifecycle APIs are ready (start/stop/reset/destroy/query).
11. Scoreboard permission controls are ready (auth required + private contest guard).
12. Docker compose orchestration integration for real instance bring-up/teardown is ready.
13. Runtime lifecycle E2E verification is ready (`start -> stop -> reset -> destroy`, real container state consistent).
14. Scoreboard websocket auth fallback for browser is ready (`Authorization` or `access_token` query).
15. Admin API v2 is ready (challenge CRUD-lite, contest create/edit/status, contest-challenge binding CRUD, instance list).
16. Admin frontend v2 is ready (challenge create/visibility, contest create/status, contest-challenge binding/sort, instance monitor).
17. Next: richer runtime monitor/notifications + operation audit API/UI.

## Frontend

1. API client layer and auth state persistence are ready.
2. Player pages are ready: login/register, contest list, challenge detail, submission panel, instance controls.
3. Scoreboard websocket consumer and polling fallback are ready.
4. Admin v2 page is ready: challenge management, contest management, challenge binding management, instance monitor.
5. Next: admin detail editors (challenge full metadata editor, contest advanced config) and operation audit views.

## Infrastructure

1. Add Makefile for local bootstrap and migrations.
2. Add CI workflow for Rust check + frontend typecheck.
3. Add production compose and split env templates.
