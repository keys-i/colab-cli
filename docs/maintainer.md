# Maintainer

Private release helpers are maintainer conveniences, not user commands.

```sh
COLAB_CLI_DEV=1 COLAB_CLI_MAINTAINER=1 cargo run --features dev-tools -- settings dev release name
```

Access requires a compiled dev-tools build, an explicit dev switch (`COLAB_CLI_DEV=1` or `[dev].enabled = true`), and `COLAB_CLI_MAINTAINER=1` or a configured maintainer identity. Normal users see:

```text
private maintainer command
```

Public release planning stays in Shipyard and CI.
