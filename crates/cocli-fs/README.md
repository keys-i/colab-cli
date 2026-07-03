# cocli-fs

Manifest and sync-planning utilities for local and Colab files.

Default excludes cover `.git`, `target`, `__pycache__`, `.ipynb_checkpoints`, `node_modules`, `.venv`, `.env`, and common checkpoint directories. Hashing is explicit so normal scans use path, size, mtime, and executable bit.
