# Public Vs Experimental

Classification after the July 4, 2026 convergence pass.

| Command | Decision | Why | Replacement |
|---|---|---|---|
| `colab session` | public | Core session lifecycle. | n/a |
| `colab run` | public | Core execution surface. | n/a |
| `colab fs` | public | Core file and Drive surface. | n/a |
| `colab status` | public | Cheap state and runtime checks. | n/a |
| `colab auth` | public | Sign-in and credential inspection. | n/a |
| `colab log` | public | Familiar history/export entry point from the reference CLI. | n/a |
| `colab settings` | public | Local config, UI, support, experiments. | n/a |
| `colab ai` | public | Agent-facing wrappers only. | n/a |
| `colab update` | public | Local update check/install entry point. | n/a |
| `colab version` | public | Simple version command plus `-V`. | n/a |
| `colab pay` | public | Opens Colab billing/compute page; no invented billing data. | n/a |
| `colab completions` | public | Shell integration. | n/a |
| `colab continue` | experiment | Checkpoint/replay is useful but optional. | Enable in `colab settings experiments`. |
| `colab distribute` | experiment | Recipes, pools, and shards are opt-in. | Enable in `colab settings experiments`. |
| `colab slurp` | hidden alias | Old recipe name. | `colab distribute recipe`. |
| `colab fleet` | hidden alias | Old pool planning name. | `colab distribute pool`. |
| `colab exec` | hidden alias | Old execution surface. | `colab run`. |
| `colab env` | hidden alias | Old package surface. | `colab run pip` or `colab run pkg`. |
| `colab mount` | hidden alias | Old Drive mount surface. | `colab fs drive`. |
| `colab runtime` | hidden alias | Old runtime status surface. | `colab status runtime`. |
| `colab tools` | hidden alias | Old tool catalog. | `colab ai tools`. |
| `colab config` | hidden alias | Old settings surface. | `colab settings`. |
| `colab doctor` | hidden alias | Avoids another diagnostic noun. | `colab status check`. |
| `colab agent` | hidden alias | Merged into AI. | `colab ai`. |
| `colab release` | maintainer-only removed | Release helpers are private. | `colab settings dev release` with dev feature and maintainer gate. |
| `colab run julia` | hidden parser path | Do not advertise language tools unless kernel metadata justifies it. | `colab run pkg`. |
| `colab run r` | hidden parser path | Do not advertise language tools unless kernel metadata justifies it. | `colab run pkg`. |
