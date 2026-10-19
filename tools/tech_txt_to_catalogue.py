#!/usr/bin/env python3
"""Turn tech.txt into data/technology.toml.

tech.txt is the design document: thirty-five development paths, each a run of
stages, each stage a list of techniques, products, foods and examples. This
reads it and writes one `[[tech]]` entry per listing, with an id of the form
`path.stage.slug`, so that every line of the document has a name the rest of
the model can point at.

What tech.txt does not say is decided by hand and lives in the catalogue
itself, on the same entries:

    model    what the listing is in the simulation: make:<product>,
             wear:<garment>, build:<BuildingType>, do:<action> or
             born:<what an agent is born knowing>
    same_as  the same discovery listed again under another path. A matching
             name is not enough - "knives" is stone in tool.1, bronze in
             tool.5 and iron in weapon.5 - so this is said, never guessed
    needs    other listings that have to be known first
    drive    which drive the listing answers when it is not the obvious one
             ("luxury" for things worth having only once the rest is met)
    innate   stage 0 is what everybody is born knowing; `innate = false`
             says why one stage-0 listing is not (see `note`)
    note     the reason for any of the above

and the model's own things the document has no line for go in `[[unlisted]]`
with a reason. Running this again after tech.txt changes keeps all of that
(`merge_kept`); a hand-decided entry whose listing has gone from the document
stops the run rather than being dropped.

    python3 tools/tech_txt_to_catalogue.py tech.txt data/technology.toml

src/analytics/tests/technology_catalogue_tests.rs holds the catalogue to the
making chain.
"""
import os
import re
import sys
import tomllib

ROLES = {"products": "product", "food": "food", "examples": "example"}
STOP_SECTIONS = ("cross-domain dependencies", "common prerequisites", "craft/tool quality levels")
KEPT = ("model", "same_as", "needs", "drive", "innate", "note")
MOST_WORDS_IN_AN_ID = 6


def slug(text):
    text = text.lower().replace("&", "and")
    text = re.sub(r"\(.*?\)", "", text)
    text = re.sub(r"[^a-z0-9]+", "_", text).strip("_")
    return "_".join(text.split("_")[:MOST_WORDS_IN_AN_ID])


def parse(path):
    paths, current, stage = [], None, None
    role, subgroup = "technique", None
    for raw in open(path, encoding="utf-8"):
        line = raw.strip().replace("’", "'").replace("“", '"').replace("”", '"')
        if not line:
            continue
        low = line.lower()
        if low.startswith(STOP_SECTIONS):
            break
        if line.endswith("Development Path"):
            name = line[: -len("Development Path")].strip()
            current = {"id": slug(name), "name": name, "stages": []}
            paths.append(current)
            stage = None
            continue
        m = re.match(r"^Stage\s*(\d+)\s*:\s*(.+)$", line)
        if m and current is not None:
            stage = {"stage": int(m.group(1)), "title": m.group(2).strip(), "items": []}
            current["stages"].append(stage)
            role, subgroup = "technique", None
            continue
        if stage is None:
            continue
        head = low.rstrip(":").strip()
        if line.endswith(":") and head in ROLES:
            role, subgroup = ROLES[head], None
            continue
        if line.endswith(":") and len(line) < 40:
            subgroup = line.rstrip(":").strip()
            role = "technique"
            continue
        stage["items"].append({"name": line.rstrip(".").strip(), "role": role, "group": subgroup})
    return paths


def listings(paths):
    """Every listing in document order, each with its id."""
    out = []
    for p in paths:
        for s in p["stages"]:
            seen = {}
            for it in s["items"]:
                base = f"{p['id']}.{s['stage']}.{slug(it['name'])}"
                seen[base] = seen.get(base, 0) + 1
                ident = base if seen[base] == 1 else f"{base}_{seen[base]}"
                out.append({"id": ident, "path": p["id"], "stage": s["stage"], **it})
    return out


def merge_kept(entries, old):
    """Carry the hand-decided fields over from the catalogue as it stands."""
    by_id = {e["id"]: e for e in entries}
    lost = []
    for kept in old.get("tech", []):
        fields = {k: kept[k] for k in KEPT if k in kept}
        if not fields:
            continue
        if kept["id"] not in by_id:
            lost.append(kept["id"])
            continue
        by_id[kept["id"]].update(fields)
    return lost


def q(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def value(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(q(x) for x in v) + "]"
    return q(v)


HEADER = """\
# The technology catalogue: tech.txt as data.
#
# Written by tools/tech_txt_to_catalogue.py from tech.txt. Edit tech.txt and
# run the tool again for anything the document says; edit this file by hand
# only for the fields the tool keeps (model, same_as, needs, drive, innate,
# note) and for [[unlisted]]. The tool's docstring says what each field means.
# src/analytics/tests/technology_catalogue_tests.rs holds it to the making
# chain.
"""


def write(out, paths, entries, unlisted, stage_paths):
    lines = [HEADER]
    for p, model in zip(paths, stage_paths):
        lines.append("[[path]]")
        lines.append(f"id = {q(p['id'])}")
        lines.append(f"name = {q(p['name'])}")
        if model:
            lines.append(f"model = {q(model)}")
        lines.append("stages = [")
        for s in p["stages"]:
            lines.append(f"  {q(s['title'])},")
        lines.append("]")
        lines.append("")
    for e in entries:
        lines.append("[[tech]]")
        for k in ("id", "name", "path", "stage", "role", "group") + KEPT:
            if e.get(k) is not None:
                lines.append(f"{k} = {value(e[k])}")
        lines.append("")
    for u in unlisted:
        lines.append("[[unlisted]]")
        for k, v in u.items():
            lines.append(f"{k} = {value(v)}")
        lines.append("")
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(lines).rstrip("\n") + "\n")


def main(src, out):
    paths = parse(src)
    entries = listings(paths)
    old = {}
    if os.path.exists(out):
        with open(out, "rb") as f:
            old = tomllib.load(f)
    lost = merge_kept(entries, old)
    if lost:
        sys.exit("hand-decided entries whose listing is no longer in the document:\n  " + "\n  ".join(lost))
    old_models = {p["id"]: p.get("model") for p in old.get("path", [])}
    stage_paths = [old_models.get(p["id"]) for p in paths]
    write(out, paths, entries, old.get("unlisted", []), stage_paths)
    n_stages = sum(len(p["stages"]) for p in paths)
    linked = sum(1 for e in entries if "model" in e)
    print(f"{len(paths)} paths, {n_stages} stages, {len(entries)} listings, {linked} linked to the model", file=sys.stderr)


if __name__ == "__main__":
    main(*sys.argv[1:])
