#!/usr/bin/env python3
"""Fails if a relative link in a Markdown file points at a file or folder that does not exist.

Checks the root, docs/, crates/*/ and scripts/ Markdown files. External links and in-page anchors are ignored.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = [*ROOT.glob("*.md"), *ROOT.glob("docs/**/*.md"), *ROOT.glob("crates/*/README.md"), *ROOT.glob("scripts/*.md"),
         *ROOT.glob(".github/*.md")]
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
bad = []
for f in FILES:
    in_code = False
    for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
        if line.lstrip().startswith("```"):
            in_code = not in_code
        if in_code:
            continue
        for target in LINK.findall(line):
            if re.match(r"^(https?:|mailto:|#)", target):
                continue
            path = target.split("#")[0]
            if path and not (f.parent / path).exists():
                bad.append(f"{f.relative_to(ROOT)}:{n}: broken link {target}")
if bad:
    print("\n".join(bad))
    sys.exit(1)
print(f"links ok ({len(FILES)} files)")
