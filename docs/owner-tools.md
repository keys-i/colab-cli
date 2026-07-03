# Owner Tools

Release helpers are private maintainer conveniences.

They are not compiled by default and do not appear in normal help:

```sh
cargo run -- --help
```

Build them explicitly:

```sh
cargo run --features owner-tools -- settings owner release name
```

There is a soft local gate:

```sh
COLAB_CLI_OWNER=keys
```

The gate only avoids accidental use on the wrong machine. It is not a security boundary.

Normal users should not need these commands. Public release work is handled by Shipyard and CI.
