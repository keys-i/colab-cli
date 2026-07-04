# Colabtools Feature Map

Reference checked: `googlecolab/colabtools` local checkout under `colabtools/`. The repo is a reference for public Colab concepts, not code to vendor.

| Colabtools feature | CLI equivalent | Status | Reason |
|---|---|---|---|
| `google.colab.drive.mount` | `colab-cli fs drive mount` | wrapped through kernel cell | Drive mount needs Colab kernel/browser approval. CLI shows progress and maps common failures. |
| `google.colab.drive.flush_and_unmount` | `colab-cli fs drive unmount` | wrapped through kernel cell | CLI exposes Drive lifecycle without copying internals. |
| Drive mount status | `colab-cli fs drive status` | implemented | Checks mounted state through the selected session. |
| `google.colab.auth.authenticate_user` | `colab-cli auth login` | CLI equivalent exists | CLI auth uses profile commands and redacted status. |
| auth status/export | `colab-cli auth status`, `auth export-redacted` | implemented | Secrets are redacted by default. |
| `google.colab.files.upload/download` | `colab-cli fs push`, `fs pull`, `fs sync`, `fs changed` | CLI equivalent exists | Terminal file transfer is explicit path-based, not browser picker based. |
| output/log capture | `colab-cli run ...`, `logs/export` where available | implemented/deferred by command | Run output streams normally; richer log export is kept as explicit support work. |
| runtime/backend info | `colab-cli status runtime --backend`, `--versions`, `--gpu`, `--tpu` | implemented | Uses public runtime/session surfaces. Backend package snapshots are reference data only. |
| notebook execution | `colab-cli run notebook` | implemented | Runs notebooks through the selected runtime path. |
| user data/secrets | `settings support bug-report`, redaction, future secrets check | partial | CLI must not print secret values. Only redacted checks are appropriate. |
| forms/widgets | none | not applicable outside notebook UI | Terminal widgets would be fake unless backed by a real CLI workflow. |
| JavaScript/browser helpers | none | not applicable outside notebook UI | Browser-only APIs stay in notebooks. |
| data tables/quick charts/autoviz | `ai code deps`, future explain-only helpers | deferred | Useful terminal equivalent would be static inspection, not browser rendering. |
| BigQuery/Sheets helpers | future explicit cloud commands | deferred | Needs separate auth/scopes and should not be hidden behind Colab session commands. |
| resource monitor | `status runtime --all`, future live checks | partial | Cheap local status stays default; live checks are experimental. |
| Colab AI/code assistance concepts | `run ast`, `ai plan`, `ai audit`, `ai code` | implemented/gated | Inspectable local tools only; no hidden model calls. |
| `_message.blocking_request` kernel channel | Drive/status kernel operations | wrapped through kernel cell | Normal errors are mapped; raw tracebacks appear only with `--verbose`. |
| import hooks/magics | none | not CLI-appropriate | Colab notebook import behavior should not be copied into the terminal. |
| HTML/background server helpers | none | not CLI-appropriate | These are notebook display/browser features. |

Implemented missing useful equivalents in this pass:

- Drive mount/status progress and friendly errors
- `run pip ...` package surface
- local AST/code observer under `run`
- `distribute` recipe/pool/shard surface, gated
- redacted settings/support surfaces

Deferred:

- full MCP stdio server
- exact Tree-sitter AST
- BigQuery/Sheets terminal commands
- richer standalone log export

Not implemented:

- browser-only JavaScript helpers
- notebook forms/widgets as fake terminal widgets
- private Colab internals
- account rotation or quota-bypass behavior
