# AST Observer

The AST observer is experimental and off by default.

```sh
colab settings experiments set ast-observer true
colab run ast file.py
colab run ast file.jl
colab run ast file.R
colab run ast notebook.ipynb
colab run watch file.py --ast
colab ai code explain file.py
colab ai code deps file.py
colab run script file.py --ast --session work
colab run notebook notebook.ipynb --ast --session work
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
