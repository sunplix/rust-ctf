# Init Next Steps

## Backend

1. Core migration is ready (users, teams, contests, challenges, submissions, instances).
2. DB pool and Redis manager initialization is ready in `AppState`.
3. Basic auth endpoints are ready: register, login, refresh, me.
4. Bearer auth extractor (route guard) is ready for protected APIs.
5. Basic contest/challenge list APIs and submission flow are ready.
6. Submission rate limiting and scoreboard API are ready.
7. Dynamic flag verifier and hashed static flag check are ready.
8. Next: script-based flag verifier and scoreboard websocket push.

## Frontend

1. Add API client layer and auth state persistence.
2. Build player pages: contest list, challenge list/detail, submission panel.
3. Build admin pages: challenge management, contest control, instance monitor.
4. Add websocket service for scoreboard and runtime events.

## Infrastructure

1. Add Makefile for local bootstrap and migrations.
2. Add CI workflow for Rust check + frontend typecheck.
3. Add production compose and split env templates.
