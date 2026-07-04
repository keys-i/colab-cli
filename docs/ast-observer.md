# AST Observer

The AST observer is experimental and off by default.

```sh
colab-cli settings experiments set ast-observer true
colab-cli run ast file.py
colab-cli run ast file.jl
colab-cli run ast file.R
colab-cli run ast notebook.ipynb
colab-cli run watch file.py --ast
colab-cli ai code explain file.py
colab-cli ai code deps file.py
colab-cli run script file.py --ast --session work
colab-cli run notebook notebook.ipynb --ast --session work
```

The observer reads local files only. It does not execute code during parsing and does not send source to an AI model.

Current parser:

- Python files
- Julia files through a basic outline parser
- R files through a basic outline parser
- notebook code cells from `.ipynb`
- imports
- functions
- classes
- main guard
- top-level calls
- simple shell escape markers
- likely package dependencies from imports

This release uses small local parsers instead of Tree-sitter. Python has the
most complete outline. Julia and R support imports/packages and top-level
function/module-style markers only. Add Tree-sitter when the AST view needs
exact syntax nodes or deeper multi-language support.

JSON output is available with `--json` and contains no ANSI.
