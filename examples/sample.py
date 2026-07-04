import os
from pathlib import Path


class Job:
    def __init__(self, root: str = ".") -> None:
        self.root = Path(root)


def main() -> None:
    print(os.getcwd())


if __name__ == "__main__":
    main()
