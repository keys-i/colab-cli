# Missing Features

| Project | Missing feature | User pain | Competitor has it? | Add/defer/remove | Reason |
|---|---|---|---|---|---|
| cocli | remote manifest cache for no-op sync | cannot prove unchanged-tree transfer savings locally | rsync-like tools do | defer | needs real remote cache/API test |
| cocli | `config locate` | users search for config path wording | n/a | add | cheap alias to existing `config path` |
| cocli | `fs sync --explain` | risky sync needs plain-language preview | rsync has dry-run output | add | no new sync engine |
| cocli | `logs tail` | long jobs need live logs | google-colab-cli has log export | defer | needs real runtime stream test |
| cocli | `artifacts pull --latest` | users want latest outputs | manual workflows do | defer | continuation artifacts need stable latest index |
| Shipyard | `plan --why` | users need to trust version bumps | release-plz explains via PR context | add | reuses plan |
| Shipyard | `version --why` | users need bump reason without updating files | cargo-release is local but less PR-driven | add | reuses plan |
| Shipyard | `safety` | publish blockers should be visible | manual checklist | add | reuses safety gates |
| Shipyard | `rollback-plan` | releases need recovery path | manual docs | add | plain text, no automation |
| Shipyard | full GitHub API client | possible PR automation | release-plz has hosted support | defer | `gh` covers current need |
