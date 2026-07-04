# Troubleshooting

Use `-v` first:

```sh
colab -v status
colab -vv fs drive mount
```

If Drive mount fails before kernel checks, refresh or replace the runtime:

```sh
colab session list --refresh
colab session new --name work
```

JSON mode stays machine-readable:

```sh
colab --json -v status
```

Debug lines go to stderr, so redirect them separately when filing a bug report:

```sh
colab -vv fs drive mount 2>colab-debug.log
```
