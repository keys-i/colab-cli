# Troubleshooting

Use `-v` first:

```sh
colab-cli -v status
colab-cli -vv fs drive mount
```

If Drive mount fails before kernel checks, refresh or replace the runtime:

```sh
colab-cli session list --refresh
colab-cli session new --name work
```

JSON mode stays machine-readable:

```sh
colab-cli --json -v status
```

Debug lines go to stderr, so redirect them separately when filing a bug report:

```sh
colab-cli -vv fs drive mount 2>cocli-debug.log
```
