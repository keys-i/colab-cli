# Maintainer

Private release helpers are maintainer conveniences, not user commands.

```sh
cargo run --features dev-tools -- settings dev release name
```

Access requires `COLAB_CLI_MAINTAINER=1` or a configured maintainer identity. Normal users see:

```text
private maintainer command
```

Public release planning stays in Shipyard and CI.
