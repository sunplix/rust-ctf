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
17. Runtime alert notification API is ready (`/admin/runtime/alerts`, `scan`, `ack`, `resolve`).
18. Runtime alert scheduled scanner is ready (startup background task + configurable interval).
19. Instance expired reaper is ready (startup background task + configurable interval/batch).
20. Instance resource quota enforcement and heartbeat API are ready (`INSTANCE_DEFAULT_CPU_LIMIT`, `INSTANCE_DEFAULT_MEMORY_LIMIT_MB`, `POST /instances/heartbeat`).
21. Stale-heartbeat remediation policy is ready (configurable threshold + optional auto-reaper, default off).
22. Runtime heartbeat reporter integration is ready (`POST /instances/heartbeat/report` + compose placeholders + reporter script template).
23. Runtime alert UI integration is ready (admin runtime alert list/filter/scan/ack/resolve).
24. Stale-heartbeat remediation runbook is ready (`docs/STALE_HEARTBEAT_REMEDIATION_RUNBOOK.md`).
25. Compose variable schema validation is ready (`{{VAR:NAME}}` + `metadata.compose_variables` validation).
26. Instance failure self-healing strategy is ready (start/reset failure auto retry with `down` + `up --force-recreate`).
27. Template schema examples are ready (`README.md` section 8.9).
28. Challenge library runtime-template lint tooling is ready (`GET /admin/challenges/runtime-template/lint` + `backend/scripts/runtime/challenge_template_lint.sh`).
29. Runtime-template lint is integrated in admin UI (challenge module sub-tab: 模板校验).
30. Next: add CI health/lint pipeline and failure-threshold policy for lint reports.

## Frontend

1. API client layer and auth state persistence are ready.
2. Player pages are ready: login/register, contest list, challenge detail, submission panel, instance controls.
3. Scoreboard websocket consumer and polling fallback are ready.
4. Admin v2 page is ready: challenge management, contest management, challenge binding management, instance/runtime monitor.
5. Next: admin detail editors (challenge full metadata editor, contest advanced config) and operation runbook shortcuts.

## Infrastructure

1. Add Makefile for local bootstrap and migrations.
2. Add CI workflow for Rust check + frontend typecheck.
3. Add production compose and split env templates.
