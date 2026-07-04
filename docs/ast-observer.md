# AST Observer

The AST observer is experimental and off by default.

```sh
colab-cli settings experiments set ast-observer true
colab-cli ai ast file.py
colab-cli ai ast watch file.py
colab-cli ai code explain file.py
colab-cli ai code deps file.py
colab-cli run script file.py --ast --session work
colab-cli run notebook notebook.ipynb --ast --session work
```

The observer reads local files only. It does not execute code during parsing and does not send source to an AI model.

Current parser:

- Python files
- notebook code cells from `.ipynb`
- imports
- functions
- classes
- main guard
- top-level calls
- simple shell escape markers
- likely package dependencies from imports

This release uses a small local parser instead of Tree-sitter. It is good enough for an outline, not for semantic refactoring. Add Tree-sitter when the AST view needs exact syntax nodes or multi-language support.

JSON output is available with `--json` and contains no ANSI.
