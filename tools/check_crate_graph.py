#!/usr/bin/env python3
"""Validate the Moqentra crate dependency graph against architecture rules."""

from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore


def load_toml(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def direct_dependencies(manifest: dict) -> set[str]:
    """Return direct crate dependencies from a Cargo.toml manifest."""
    deps: set[str] = set()
    raw = manifest.get("dependencies", {})
    for key, value in raw.items():
        if isinstance(value, dict):
            # workspace inheritance is allowed; still a dependency
            deps.add(key)
        elif isinstance(value, str):
            deps.add(key)
        else:
            deps.add(key)
    return deps


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    rules_path = root / "tools" / "crate_graph_rules.json"
    rules = json.loads(rules_path.read_text())

    crates_dir = root / "crates"
    apps_dir = root / "apps"

    manifests: dict[str, dict] = {}
    locations: dict[str, Path] = {}

    for d in [crates_dir, apps_dir]:
        if not d.exists():
            continue
        for sub in d.iterdir():
            cargo_toml = sub / "Cargo.toml"
            if not cargo_toml.exists():
                continue
            manifest = load_toml(cargo_toml)
            name = manifest["package"]["name"]
            manifests[name] = manifest
            locations[name] = cargo_toml

    violations: list[str] = []
    all_crates = set(manifests.keys())

    for name, manifest in manifests.items():
        deps = direct_dependencies(manifest)
        deps = deps & all_crates  # only internal workspace crates matter here
        rule_key = None
        if name in rules["crates"]:
            rule_key = "crates"
        elif name in rules["apps"]:
            rule_key = "apps"
        else:
            violations.append(f"{name}: no architecture rule defined")
            continue

        rule = rules[rule_key][name]
        allowed = set(rule["allowed_internal"])
        forbidden_external = set(rule.get("forbidden_external", []))

        for dep in deps:
            if dep not in allowed:
                violations.append(
                    f"{name}: dependency on {dep} is not allowed by architecture rules"
                )

        if rule_key == "crates":
            raw_deps = manifest.get("dependencies", {})
            for dep_name in raw_deps:
                if dep_name in forbidden_external:
                    violations.append(
                        f"{name}: forbidden external dependency {dep_name}"
                    )

    # Adapter-to-adapter ban
    if rules.get("adapter_adapter_ban", False):
        adapters = set(rules["adapter_crates"])
        for name, manifest in manifests.items():
            if name not in adapters:
                continue
            deps = direct_dependencies(manifest) & adapters
            deps.discard(name)
            for dep in deps:
                violations.append(
                    f"{name}: adapters must not depend on each other ({dep})"
                )

    if violations:
        print("Crate graph violations:")
        for v in violations:
            print(f"  - {v}")
        return 1

    print("Crate graph OK: all {} crates/apps conform to architecture rules.".format(len(manifests)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
