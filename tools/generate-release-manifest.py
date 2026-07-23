#!/usr/bin/env python3
"""Build a ReleaseManifest JSON with real report paths (R1-PKG-004).

Usage:
  python3 tools/generate-release-manifest.py \
    --version 0.1.0 --commit $(git rev-parse HEAD) \
    --sbom artifacts/r1-evidence/sbom/sbom.spdx.json \
    --out artifacts/r1-evidence/ReleaseManifest.json \
    control-plane=moqentra/control-plane@sha256:abc \
    scheduler=moqentra/scheduler@sha256:def
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def parse_artifact(spec: str) -> tuple[str, str, str]:
    # name=image@digest or name=image:tag
    if "=" not in spec:
        raise SystemExit(f"artifact must be name=image@digest: {spec}")
    name, rest = spec.split("=", 1)
    if "@" in rest:
        image, digest = rest.rsplit("@", 1)
        if not digest.startswith("sha256:"):
            digest = f"sha256:{digest}" if not digest.startswith("sha256") else digest
    elif ":" in rest:
        image, tag = rest.rsplit(":", 1)
        digest = f"tag:{tag}"
    else:
        image, digest = rest, "unknown"
    return name, image, digest


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--version", required=True)
    p.add_argument("--commit", required=True)
    p.add_argument("--sbom", default="")
    p.add_argument("--provenance", default="")
    p.add_argument("--out", required=True)
    p.add_argument("artifacts", nargs="*", help="name=image@sha256:...")
    args = p.parse_args()

    artifacts = {}
    for a in args.artifacts:
        name, image, digest = parse_artifact(a)
        artifacts[name] = {
            "image": image,
            "digest": digest,
            "architectures": ["amd64"],
            "platform_tiers": {"linux-x86_64": "Certified"},
        }

    # Reject boolean placeholders for report refs.
    sbom = args.sbom or "missing"
    prov = args.provenance or "missing"
    if sbom in ("true", "false", "1", "0"):
        raise SystemExit("sbom_reference must be a path/URI, not a boolean")
    if prov in ("true", "false", "1", "0"):
        raise SystemExit("provenance_reference must be a path/URI, not a boolean")

    manifest = {
        "version": args.version,
        "commit": args.commit,
        "artifacts": artifacts,
        "sbom_reference": sbom,
        "provenance_reference": prov,
        "signatures": {},
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
