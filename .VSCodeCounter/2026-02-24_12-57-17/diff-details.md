# Diff Details

Date : 2026-02-24 12:57:17

Directory /Users/sunplix/rust-ctf/backend

Total : 34 files,  3616 codes, 16 comments, 597 blanks, all 4229 lines

[Summary](results.md) / [Details](details.md) / [Diff Summary](diff.md) / Diff Details

## Files
| filename | language | code | comment | blank | total |
| :--- | :--- | ---: | ---: | ---: | ---: |
| [backend/Dockerfile](/backend/Dockerfile) | Docker | 21 | 0 | 6 | 27 |
| [backend/migrations/20260214124500_init_core_schema.sql](/backend/migrations/20260214124500_init_core_schema.sql) | MS SQL | 143 | 0 | 24 | 167 |
| [backend/migrations/20260215153000_add_audit_logs.sql](/backend/migrations/20260215153000_add_audit_logs.sql) | MS SQL | 16 | 0 | 4 | 20 |
| [backend/migrations/20260215193000_add_team_invitations.sql](/backend/migrations/20260215193000_add_team_invitations.sql) | MS SQL | 24 | 0 | 7 | 31 |
| [backend/migrations/20260215204000_enhance_challenge_management.sql](/backend/migrations/20260215204000_enhance_challenge_management.sql) | MS SQL | 58 | 0 | 8 | 66 |
| [backend/migrations/20260215213000_add_challenge_status.sql](/backend/migrations/20260215213000_add_challenge_status.sql) | MS SQL | 20 | 0 | 6 | 26 |
| [backend/migrations/20260215223000_add_contest_announcements_and_dynamic_scoring.sql](/backend/migrations/20260215223000_add_contest_announcements_and_dynamic_scoring.sql) | MS SQL | 29 | 0 | 8 | 37 |
| [backend/migrations/20260215233000_add_runtime_alerts.sql](/backend/migrations/20260215233000_add_runtime_alerts.sql) | MS SQL | 32 | 0 | 7 | 39 |
| [backend/migrations/20260218090000_add_contest_poster_fields.sql](/backend/migrations/20260218090000_add_contest_poster_fields.sql) | MS SQL | 5 | 0 | 3 | 8 |
| [backend/migrations/20260219190000_add_auth_email_verification_and_password_reset.sql](/backend/migrations/20260219190000_add_auth_email_verification_and_password_reset.sql) | MS SQL | 39 | 0 | 9 | 48 |
| [backend/migrations/20260219194000_add_site_settings.sql](/backend/migrations/20260219194000_add_site_settings.sql) | MS SQL | 19 | 0 | 5 | 24 |
| [backend/migrations/20260220001000_add_challenge_categories_and_blood_bonus.sql](/backend/migrations/20260220001000_add_challenge_categories_and_blood_bonus.sql) | MS SQL | 46 | 0 | 9 | 55 |
| [backend/migrations/20260220090000_add_builtin_pwn_category.sql](/backend/migrations/20260220090000_add_builtin_pwn_category.sql) | MS SQL | 9 | 0 | 3 | 12 |
| [backend/migrations/20260221093000_add_challenge_attachment_max_bytes_to_site_settings.sql](/backend/migrations/20260221093000_add_challenge_attachment_max_bytes_to_site_settings.sql) | MS SQL | 9 | 0 | 5 | 14 |
| [backend/migrations/20260221162000_add_contest_registrations_and_time_mode_and_challenge_hints.sql](/backend/migrations/20260221162000_add_contest_registrations_and_time_mode_and_challenge_hints.sql) | MS SQL | 40 | 0 | 11 | 51 |
| [backend/migrations/20260224112000_add_scoreboard_channels.sql](/backend/migrations/20260224112000_add_scoreboard_channels.sql) | MS SQL | 39 | 0 | 10 | 49 |
| [backend/scripts/m5/full_acceptance.sh](/backend/scripts/m5/full_acceptance.sh) | Shell Script | 131 | 8 | 29 | 168 |
| [backend/scripts/m5/load_benchmark.sh](/backend/scripts/m5/load_benchmark.sh) | Shell Script | 237 | 1 | 28 | 266 |
| [backend/scripts/m5/security_smoke.sh](/backend/scripts/m5/security_smoke.sh) | Shell Script | 202 | 1 | 35 | 238 |
| [backend/scripts/runtime/challenge_template_lint.sh](/backend/scripts/runtime/challenge_template_lint.sh) | Shell Script | 35 | 1 | 7 | 43 |
| [backend/scripts/runtime/heartbeat_reporter.sh](/backend/scripts/runtime/heartbeat_reporter.sh) | Shell Script | 11 | 1 | 4 | 16 |
| [backend/scripts/runtime/runtime_full_regression.sh](/backend/scripts/runtime/runtime_full_regression.sh) | Shell Script | 106 | 1 | 16 | 123 |
| [backend/scripts/runtime/scoreboard_ws_smoke.mjs](/backend/scripts/runtime/scoreboard_ws_smoke.mjs) | JavaScript | 182 | 0 | 27 | 209 |
| [backend/scripts/runtime/single_image_smoke.sh](/backend/scripts/runtime/single_image_smoke.sh) | Shell Script | 102 | 1 | 19 | 122 |
| [backend/scripts/runtime/wireguard_smoke.sh](/backend/scripts/runtime/wireguard_smoke.sh) | Shell Script | 112 | 1 | 21 | 134 |
| [backend/scripts/verifiers/simple_compare.sh](/backend/scripts/verifiers/simple_compare.sh) | Shell Script | 17 | 1 | 6 | 24 |
| [backend/src/auth.rs](/backend/src/auth.rs) | Rust | 304 | 0 | 54 | 358 |
| [backend/src/config.rs](/backend/src/config.rs) | Rust | 129 | 0 | 4 | 133 |
| [backend/src/error.rs](/backend/src/error.rs) | Rust | 96 | 0 | 13 | 109 |
| [backend/src/mailer.rs](/backend/src/mailer.rs) | Rust | 95 | 0 | 15 | 110 |
| [backend/src/main.rs](/backend/src/main.rs) | Rust | 212 | 0 | 29 | 241 |
| [backend/src/password_policy.rs](/backend/src/password_policy.rs) | Rust | 294 | 0 | 47 | 341 |
| [backend/src/runtime_template.rs](/backend/src/runtime_template.rs) | Rust | 632 | 0 | 96 | 728 |
| [backend/src/state.rs](/backend/src/state.rs) | Rust | 170 | 0 | 22 | 192 |

[Summary](results.md) / [Details](details.md) / [Diff Summary](diff.md) / Diff Details