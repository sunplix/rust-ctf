# Diff Details

Date : 2026-02-24 12:58:04

Directory /Users/sunplix/rust-ctf/frontend

Total : 72 files,  5569 codes, -10 comments, -1606 blanks, all 3953 lines

[Summary](results.md) / [Details](details.md) / [Diff Summary](diff.md) / Diff Details

## Files
| filename | language | code | comment | blank | total |
| :--- | :--- | ---: | ---: | ---: | ---: |
| [backend/Dockerfile](/backend/Dockerfile) | Docker | -21 | 0 | -6 | -27 |
| [backend/migrations/20260214124500_init_core_schema.sql](/backend/migrations/20260214124500_init_core_schema.sql) | MS SQL | -143 | 0 | -24 | -167 |
| [backend/migrations/20260215153000_add_audit_logs.sql](/backend/migrations/20260215153000_add_audit_logs.sql) | MS SQL | -16 | 0 | -4 | -20 |
| [backend/migrations/20260215193000_add_team_invitations.sql](/backend/migrations/20260215193000_add_team_invitations.sql) | MS SQL | -24 | 0 | -7 | -31 |
| [backend/migrations/20260215204000_enhance_challenge_management.sql](/backend/migrations/20260215204000_enhance_challenge_management.sql) | MS SQL | -58 | 0 | -8 | -66 |
| [backend/migrations/20260215213000_add_challenge_status.sql](/backend/migrations/20260215213000_add_challenge_status.sql) | MS SQL | -20 | 0 | -6 | -26 |
| [backend/migrations/20260215223000_add_contest_announcements_and_dynamic_scoring.sql](/backend/migrations/20260215223000_add_contest_announcements_and_dynamic_scoring.sql) | MS SQL | -29 | 0 | -8 | -37 |
| [backend/migrations/20260215233000_add_runtime_alerts.sql](/backend/migrations/20260215233000_add_runtime_alerts.sql) | MS SQL | -32 | 0 | -7 | -39 |
| [backend/migrations/20260218090000_add_contest_poster_fields.sql](/backend/migrations/20260218090000_add_contest_poster_fields.sql) | MS SQL | -5 | 0 | -3 | -8 |
| [backend/migrations/20260219190000_add_auth_email_verification_and_password_reset.sql](/backend/migrations/20260219190000_add_auth_email_verification_and_password_reset.sql) | MS SQL | -39 | 0 | -9 | -48 |
| [backend/migrations/20260219194000_add_site_settings.sql](/backend/migrations/20260219194000_add_site_settings.sql) | MS SQL | -19 | 0 | -5 | -24 |
| [backend/migrations/20260220001000_add_challenge_categories_and_blood_bonus.sql](/backend/migrations/20260220001000_add_challenge_categories_and_blood_bonus.sql) | MS SQL | -46 | 0 | -9 | -55 |
| [backend/migrations/20260220090000_add_builtin_pwn_category.sql](/backend/migrations/20260220090000_add_builtin_pwn_category.sql) | MS SQL | -9 | 0 | -3 | -12 |
| [backend/migrations/20260221093000_add_challenge_attachment_max_bytes_to_site_settings.sql](/backend/migrations/20260221093000_add_challenge_attachment_max_bytes_to_site_settings.sql) | MS SQL | -9 | 0 | -5 | -14 |
| [backend/migrations/20260221162000_add_contest_registrations_and_time_mode_and_challenge_hints.sql](/backend/migrations/20260221162000_add_contest_registrations_and_time_mode_and_challenge_hints.sql) | MS SQL | -40 | 0 | -11 | -51 |
| [backend/migrations/20260224112000_add_scoreboard_channels.sql](/backend/migrations/20260224112000_add_scoreboard_channels.sql) | MS SQL | -39 | 0 | -10 | -49 |
| [backend/scripts/m5/full_acceptance.sh](/backend/scripts/m5/full_acceptance.sh) | Shell Script | -131 | -8 | -29 | -168 |
| [backend/scripts/m5/load_benchmark.sh](/backend/scripts/m5/load_benchmark.sh) | Shell Script | -237 | -1 | -28 | -266 |
| [backend/scripts/m5/security_smoke.sh](/backend/scripts/m5/security_smoke.sh) | Shell Script | -202 | -1 | -35 | -238 |
| [backend/scripts/runtime/challenge_template_lint.sh](/backend/scripts/runtime/challenge_template_lint.sh) | Shell Script | -35 | -1 | -7 | -43 |
| [backend/scripts/runtime/heartbeat_reporter.sh](/backend/scripts/runtime/heartbeat_reporter.sh) | Shell Script | -11 | -1 | -4 | -16 |
| [backend/scripts/runtime/runtime_full_regression.sh](/backend/scripts/runtime/runtime_full_regression.sh) | Shell Script | -106 | -1 | -16 | -123 |
| [backend/scripts/runtime/scoreboard_ws_smoke.mjs](/backend/scripts/runtime/scoreboard_ws_smoke.mjs) | JavaScript | -182 | 0 | -27 | -209 |
| [backend/scripts/runtime/single_image_smoke.sh](/backend/scripts/runtime/single_image_smoke.sh) | Shell Script | -102 | -1 | -19 | -122 |
| [backend/scripts/runtime/wireguard_smoke.sh](/backend/scripts/runtime/wireguard_smoke.sh) | Shell Script | -112 | -1 | -21 | -134 |
| [backend/scripts/verifiers/simple_compare.sh](/backend/scripts/verifiers/simple_compare.sh) | Shell Script | -17 | -1 | -6 | -24 |
| [backend/src/auth.rs](/backend/src/auth.rs) | Rust | -304 | 0 | -54 | -358 |
| [backend/src/config.rs](/backend/src/config.rs) | Rust | -129 | 0 | -4 | -133 |
| [backend/src/error.rs](/backend/src/error.rs) | Rust | -96 | 0 | -13 | -109 |
| [backend/src/mailer.rs](/backend/src/mailer.rs) | Rust | -95 | 0 | -15 | -110 |
| [backend/src/main.rs](/backend/src/main.rs) | Rust | -212 | 0 | -29 | -241 |
| [backend/src/password_policy.rs](/backend/src/password_policy.rs) | Rust | -294 | 0 | -47 | -341 |
| [backend/src/routes/admin.rs](/backend/src/routes/admin.rs) | Rust | -8,114 | 0 | -734 | -8,848 |
| [backend/src/routes/auth.rs](/backend/src/routes/auth.rs) | Rust | -1,271 | 0 | -170 | -1,441 |
| [backend/src/routes/contest_access.rs](/backend/src/routes/contest_access.rs) | Rust | -161 | 0 | -23 | -184 |
| [backend/src/routes/contests.rs](/backend/src/routes/contests.rs) | Rust | -708 | 0 | -69 | -777 |
| [backend/src/routes/health.rs](/backend/src/routes/health.rs) | Rust | -57 | 0 | -10 | -67 |
| [backend/src/routes/instances.rs](/backend/src/routes/instances.rs) | Rust | -3,207 | 0 | -325 | -3,532 |
| [backend/src/routes/mod.rs](/backend/src/routes/mod.rs) | Rust | -25 | 0 | -5 | -30 |
| [backend/src/routes/scoreboard.rs](/backend/src/routes/scoreboard.rs) | Rust | -1,244 | 0 | -132 | -1,376 |
| [backend/src/routes/site.rs](/backend/src/routes/site.rs) | Rust | -42 | 0 | -7 | -49 |
| [backend/src/routes/submissions.rs](/backend/src/routes/submissions.rs) | Rust | -624 | 0 | -73 | -697 |
| [backend/src/routes/teams.rs](/backend/src/routes/teams.rs) | Rust | -1,128 | 0 | -152 | -1,280 |
| [backend/src/runtime_template.rs](/backend/src/runtime_template.rs) | Rust | -632 | 0 | -96 | -728 |
| [backend/src/state.rs](/backend/src/state.rs) | Rust | -170 | 0 | -22 | -192 |
| [frontend/Dockerfile](/frontend/Dockerfile) | Docker | 7 | 0 | 4 | 11 |
| [frontend/index.html](/frontend/index.html) | HTML | 12 | 0 | 1 | 13 |
| [frontend/package-lock.json](/frontend/package-lock.json) | JSON | 1,991 | 0 | 1 | 1,992 |
| [frontend/package.json](/frontend/package.json) | JSON | 28 | 0 | 1 | 29 |
| [frontend/scripts/i18n_extract.mjs](/frontend/scripts/i18n_extract.mjs) | JavaScript | 451 | 2 | 56 | 509 |
| [frontend/src/api/client.ts](/frontend/src/api/client.ts) | TypeScript | 2,483 | 2 | 199 | 2,684 |
| [frontend/src/assets/main.css](/frontend/src/assets/main.css) | CSS | 1,143 | 0 | 192 | 1,335 |
| [frontend/src/composables/useL10n.ts](/frontend/src/composables/useL10n.ts) | TypeScript | 104 | 0 | 17 | 121 |
| [frontend/src/composables/useMarkdown.ts](/frontend/src/composables/useMarkdown.ts) | TypeScript | 50 | 0 | 11 | 61 |
| [frontend/src/composables/usePasswordStrength.ts](/frontend/src/composables/usePasswordStrength.ts) | TypeScript | 263 | 0 | 34 | 297 |
| [frontend/src/composables/useTimeFormat.ts](/frontend/src/composables/useTimeFormat.ts) | TypeScript | 45 | 0 | 11 | 56 |
| [frontend/src/env.d.ts](/frontend/src/env.d.ts) | TypeScript | 0 | 1 | 1 | 2 |
| [frontend/src/locales/README.md](/frontend/src/locales/README.md) | Markdown | 72 | 0 | 36 | 108 |
| [frontend/src/locales/catalog.json](/frontend/src/locales/catalog.json) | JSON | 9,984 | 0 | 1 | 9,985 |
| [frontend/src/locales/i18n.config.json](/frontend/src/locales/i18n.config.json) | JSON | 20 | 0 | 1 | 21 |
| [frontend/src/locales/i18n.ts](/frontend/src/locales/i18n.ts) | TypeScript | 126 | 0 | 19 | 145 |
| [frontend/src/locales/runtime.json](/frontend/src/locales/runtime.json) | JSON | 8,304 | 0 | 1 | 8,305 |
| [frontend/src/main.ts](/frontend/src/main.ts) | TypeScript | 13 | 0 | 5 | 18 |
| [frontend/src/router/index.ts](/frontend/src/router/index.ts) | TypeScript | 115 | 0 | 11 | 126 |
| [frontend/src/stores/app.ts](/frontend/src/stores/app.ts) | TypeScript | 140 | 0 | 22 | 162 |
| [frontend/src/stores/auth.ts](/frontend/src/stores/auth.ts) | TypeScript | 139 | 0 | 20 | 159 |
| [frontend/src/stores/index.ts](/frontend/src/stores/index.ts) | TypeScript | 2 | 0 | 2 | 4 |
| [frontend/src/stores/ui.ts](/frontend/src/stores/ui.ts) | TypeScript | 230 | 0 | 40 | 270 |
| [frontend/tsconfig.json](/frontend/tsconfig.json) | JSON with Comments | 20 | 0 | 1 | 21 |
| [frontend/tsconfig.node.json](/frontend/tsconfig.node.json) | JSON | 9 | 0 | 1 | 10 |
| [frontend/tsconfig.tsbuildinfo](/frontend/tsconfig.tsbuildinfo) | JSON | 1 | 0 | 0 | 1 |
| [frontend/vite.config.ts](/frontend/vite.config.ts) | TypeScript | 14 | 1 | 3 | 18 |

[Summary](results.md) / [Details](details.md) / [Diff Summary](diff.md) / Diff Details