# Usability Study

No users have been tested yet.

## Method

After each task, ask the Single Ease Question on a 1-7 scale. After the session, collect SUS-style perception feedback.

Targets:

- SUS >= 80
- SEQ >= 6 for common tasks
- 0 critical errors
- <= 1 help lookup on happy paths

## cocli Tasks

| Task | Success Criteria | Measures |
|---|---|---|
| First useful run | create/attach session, run Python, pull output | time, commands, help lookups, errors, SEQ |
| Existing session run | use last session and run command | time, command length, errors |
| File sync dry-run | see changed files before upload | time, output clarity, JSON validity |
| Resume preview | inspect latest continuation and dry-run resume | lost-work estimate, confusion points |
| Diagnose missing auth | run doctor and pick next command | time to next action, false positives |

## Shipyard Tasks

| Task | Success Criteria | Measures |
|---|---|---|
| Plan release | explain planned version bump | time, commands, help lookups, SEQ |
| Update dry-run | see files that would change | errors, output length |
| Safety check | identify publish blockers | next-action quality |
| Generate notes | produce human release notes | quality checklist |
| Rollback plan | describe recovery path | completeness, confusion points |

## Heuristic Checklist

| Heuristic | cocli Check | Shipyard Check |
|---|---|---|
| Visibility of status | doctor/plan output shows state | plan/status/safety output |
| Match user language | session, fs, continue | plan, release, publish |
| User control | dry-run and explicit `--yes` | dry-run and explicit `--yes` |
| Consistency | `<major> <command>` | flat release commands |
| Error prevention | compliance refusal, redaction | dirty-tree and PR publish refusal |
| Recognition over recall | `session last`, `config locate` | `plan --why`, `config explain` |
| Minimalism | short tables and JSON | short release output |
