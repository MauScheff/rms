from __future__ import annotations

import importlib.util
import os
import pathlib
import sys


def main() -> None:
    environment = sys.argv[1]
    reference = os.environ[environment]
    path_text, callable_name = reference.split("#", 1)
    path = pathlib.Path(path_text).resolve()
    sys.path.insert(0, str(path.parents[1] / "src"))
    spec = importlib.util.spec_from_file_location("rms_generated_proof", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load proof runner {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    getattr(module, callable_name)()


if __name__ == "__main__":
    main()
