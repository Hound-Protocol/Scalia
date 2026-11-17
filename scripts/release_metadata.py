#!/usr/bin/env python3
import argparse
import html
import json
import os
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
UPDATE_LINE = re.compile(r"^Update:\s*([A-Za-z][A-Za-z0-9 _-]*)$")
TRAILER_LINE = re.compile(r"^(update|co-authored-by|signed-off-by|claude-session|change-id):", re.I)


def git(repo, *args):
    return subprocess.run(
        ["git", *args], cwd=repo, check=True, capture_output=True, text=True
    ).stdout.strip()


def update_areas(message):
    return [
        match.group(1).strip()
        for line in message.splitlines()
        if (match := UPDATE_LINE.fullmatch(line.strip()))
    ]


def update_area(message):
    areas = set(update_areas(message))
    if len(areas) > 1:
        raise ValueError(f"multiple Update areas in one change: {', '.join(sorted(areas))}")
    return next(iter(areas), None)


def validate_pr_update(body, head_message):
    body_areas = update_areas(body)
    head_areas = update_areas(head_message)
    if len(body_areas) != 1:
        raise ValueError("PR description must contain a standalone `Update: <Area>` line")
    if len(head_areas) != 1:
        raise ValueError("the PR's final commit message must contain the same Update area")
    body_area, head_area = body_areas[0], head_areas[0]
    if body_area != head_area:
        raise ValueError(f"PR description says {body_area!r}, but the final commit says {head_area!r}")
    if body_area.casefold() == "build":
        raise ValueError("choose a specific update area; `Build` is not a release identity")
    return body_area


def commit_message(repo, ref):
    return git(repo, "show", "-s", "--format=%B", ref)


def validate_pr_head(repo, body, pr_head):
    head = pr_head
    if not head:
        try:
            head = git(repo, "rev-parse", "HEAD^2")
        except subprocess.CalledProcessError:
            head = "HEAD"
    return validate_pr_update(body, commit_message(repo, head))


def message_body(message):
    _, separator, body = message.partition("\n\n")
    if not separator:
        return ""
    lines = []
    for line in body.splitlines():
        line = line.strip()
        if TRAILER_LINE.match(line):
            continue
        if not line:
            if lines and lines[-1]:
                lines.append("")
            continue
        if not lines or not lines[-1]:
            lines.append(line)
            continue
        if line.startswith("- "):
            lines.append(line)
        else:
            lines[-1] += " " + line
    return "\n".join(line for line in lines if line)


def pull_number(message):
    for pattern in (r"\bMerge pull request #(\d+)\b", r"\(#(\d+)\)\s*$"):
        match = re.search(pattern, message, re.M)
        if match:
            return match.group(1)
    return None


def release_entries(repo):
    commits = git(repo, "rev-list", "--first-parent", "--reverse", "HEAD").splitlines()
    counts = {}
    entries = []
    for commit in commits:
        parents = git(repo, "rev-list", "--parents", "-n", "1", commit).split()[1:]
        source = parents[1] if len(parents) > 1 else commit
        source_message = commit_message(repo, source)
        mainline_message = commit_message(repo, commit)
        area = update_area(source_message) or update_area(mainline_message) or "Build"
        counts[area] = counts.get(area, 0) + 1
        number = pull_number(mainline_message)
        label = f"{area} update V {counts[area]:03d}"
        if number:
            label += f" · PR #{number}"
        subject = git(repo, "show", "-s", "--format=%s", source)
        date = git(repo, "show", "-s", "--format=%ad", "--date=format:%d %b %Y", commit)
        entries.append(
            {
                "commit": commit,
                "source": source,
                "area": area,
                "label": label,
                "pull": number,
                "subject": html.escape(subject, quote=False),
                "body": html.escape(message_body(source_message), quote=False),
                "date": date,
            }
        )
    return entries


def build_metadata(repo=ROOT):
    entries = release_entries(repo)
    if not entries or entries[-1]["area"] == "Build":
        raise ValueError("latest mainline change needs an `Update: <Area>` release identity")

    version_commit = git(repo, "log", "-1", "--format=%H", "--", "VERSION")
    base_index = next(
        (
            index
            for index, entry in enumerate(entries)
            if version_commit in (entry["commit"], entry["source"])
        ),
        -1,
    )
    patch = sum(entry["area"] != "Build" for entry in entries[base_index + 1 :])
    series = (Path(repo) / "VERSION").read_text(encoding="utf-8").splitlines()[0].strip()
    latest = entries[-1]
    return {
        "v": git(repo, "rev-parse", "--short=7", "HEAD"),
        "n": latest["label"],
        "title": f"PR #{latest['pull']}" if latest["pull"] else "",
        "version": f"{series}.{patch}",
        "changelog": [
            {"n": entry["label"], "text": entry["subject"], "body": entry["body"], "date": entry["date"]}
            for entry in reversed(entries)
        ],
    }


def stamp(repo=ROOT):
    metadata = build_metadata(repo)
    repo = Path(repo)
    game = repo / "web/game.js"
    source = game.read_text(encoding="utf-8")
    source = source.replace("__UPDATE__", metadata["n"]).replace("__VERSION__", metadata["version"])
    game.write_text(source, encoding="utf-8")
    (repo / "web/version.json").write_text(
        json.dumps({key: metadata[key] for key in ("v", "n", "title", "version")}),
        encoding="utf-8",
    )
    (repo / "web/changelog.json").write_text(json.dumps(metadata["changelog"]), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("stamp", "validate-pr"))
    args = parser.parse_args()
    if args.command == "stamp":
        stamp()
        return
    area = validate_pr_head(
        ROOT, os.environ.get("PR_BODY", ""), os.environ.get("PR_HEAD_SHA")
    )
    print(f"PR release identity: {area}")


if __name__ == "__main__":
    main()
