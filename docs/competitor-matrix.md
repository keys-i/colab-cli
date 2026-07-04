# Competitor Matrix

| Project | Competitor | What It Does | Useful Baseline | Current Measurement |
|---|---|---|---|---|
| colab | google-colab-cli | Colab sessions, exec, files, run workflows | command count, startup, docs path | not measured locally |
| colab | colab-mcp | local agent bridge to Colab browser/session | agent tool discovery and JSON stability | docs comparison only |
| colab | manual upload/download | browser or notebook file movement | bytes transferred and steps | not measured |
| colab | Jupyter/nbconvert | notebook execution | artifact and command count | not measured |
| Shipyard | release-plz | Release PR, changelog, publish, semver-checks | release plan time and parity | binary absent |
| Shipyard | cargo-release | local crate release automation | zero-config local release steps | binary absent |
| Shipyard | git-cliff | Conventional Commit changelog | changelog quality/time | binary absent |
| Shipyard | release-please | PR-based release automation | release PR workflow | binary absent |
| Shipyard | manual cargo publish | human checklist | command count and safety gates | documented only |
