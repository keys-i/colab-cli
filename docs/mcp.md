# MCP

MCP support is disabled by default.

```sh
colab-cli ai mcp
colab-cli ai mcp serve --stdio
```

Enable the experiment first:

```sh
colab-cli settings experiments set mcp-server true
```

The current build exposes the tool catalog, but the stdio MCP server is not implemented yet. When requested it reports:

```text
MCP server not implemented yet
```

Do not rely on hidden execution through MCP. Any future MCP transport must use stable JSON schemas, avoid shell injection, redact secrets, and require confirmation or omit destructive tools.
