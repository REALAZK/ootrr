import json
import sys

KF_FILE_ADULT = "Banc/KokiriForest/KokiriForest_Adult_Dynamic.bcett.byml.zs"
KF_FILE_CHILD = "Banc/KokiriForest/KokiriForest_Child_Dynamic.bcett.byml.zs"

KF_TOP = [
    ("KF Top of Sarias Recovery Heart 1", "12317452644253046635", "9778104484775444655"),
    ("KF Top of Sarias Recovery Heart 2", "1214219023609128151", "1135865174486388723"),
    ("KF Top of Sarias Recovery Heart 3", "15227875255265438204", "12744945159394614918"),
]

KF_HOUSE = [
    ("KF Sarias House Recovery Heart 1", "11501932305561522817", "8217415107461137996"),
    ("KF Sarias House Recovery Heart 2", "4751294038826612236", "3211747475593407179"),
    ("KF Sarias House Recovery Heart 3", "13752432406165881016", "9333984718219537434"),
    ("KF Sarias House Recovery Heart 4", "16890252572970412506", "13766700305743331316"),
]

HEART_CHESTS = [
    "HF Tektite Grotto Freestanding PoH",
    "GV Crate Freestanding PoH",
    "Graveyard Dampe Race Freestanding PoH",
]

DMC_VOLCANO = ("DMC Volcano Freestanding PoH", "744856631664786545")
DMC_WALL = ("DMC Wall Freestanding PoH", "10331043360028932045")

GRAVEYARD_HASH = "2157466067026784951"
GRAVEYARD_FILES = [
    "Banc/Graveyard/Graveyard_Adult_Static.bcett.byml.zs",
    "Banc/Graveyard/Graveyard_Child_Static.bcett.byml.zs",
]


def dump(o):
    return json.dumps(o, separators=(",", ":"), ensure_ascii=False)


def load(path):
    with open(path, encoding="utf-8", newline="") as f:
        t = f.read()
    rows = t.split("\n")
    return rows


def entry_of(row):
    s = row.rstrip(",")
    if s.startswith('{"name"') or s.startswith('{"folder"'):
        return json.loads(s), row.endswith(",")
    return None, False


def kf_refs(adult, child):
    return [
        {"hash": adult, "files": [KF_FILE_ADULT]},
        {"hash": child, "files": [KF_FILE_CHILD]},
    ]


def has_graveyard(rows):
    zone = ""
    for row in rows:
        if row.startswith('"locations"'):
            zone = "locations"
        elif row.startswith('"unmatched_oot"'):
            zone = "oot"
        e, _ = entry_of(row)
        if zone == "locations" and e is not None and e.get("name") == "Graveyard Freestanding PoH":
            return True
    return False


def main(path):
    rows = load(path)
    placed = has_graveyard(rows)
    out = []
    zone = ""
    seen_graveyard = False
    for row in rows:
        if row.startswith('"locations"'):
            zone = "locations"
        elif row.startswith('"unmatched_oot"'):
            zone = "oot"
        elif row.startswith('"unmatched_remake"'):
            zone = "remake"
        e, comma = entry_of(row)
        if e is None:
            out.append(row)
            continue
        keep = True
        if zone == "locations":
            n = e["name"]
            if n in HEART_CHESTS and e["kind"] == "heart":
                e["kind"] = "chest"
            for table, pool in ((KF_TOP, "pool-KF_Top_of_Sarias_Recovery_Heart_1-3"), (KF_HOUSE, "pool-KF_Sarias_House_Recovery_Heart_1-4")):
                for name, a, c in table:
                    if n == name:
                        e["refs"] = kf_refs(a, c)
                        e["pool"] = pool
                        e["ev"] = ["pool"]
            if n == DMC_VOLCANO[0] or n == DMC_WALL[0]:
                h = DMC_VOLCANO[1] if n == DMC_VOLCANO[0] else DMC_WALL[1]
                e["refs"][0]["hash"] = h
                e["conf"] = "forced"
                e["ev"] = ["fit"]
                e.pop("pool", None)
            if n == "Graveyard Dampe Race Freestanding PoH" and not placed:
                new = {
                    "name": "Graveyard Freestanding PoH",
                    "area": "Graveyard",
                    "kind": "heart",
                    "vanilla": "Piece of Heart",
                    "item": "Obj_Heart_Piece",
                    "conf": "forced",
                    "folder": "Graveyard",
                    "via": "actor",
                    "age": "both",
                    "refs": [{"hash": GRAVEYARD_HASH, "files": GRAVEYARD_FILES}],
                    "ev": ["elim"],
                }
                out.append(dump(new) + ",")
        if zone == "oot" and e.get("name") == "Graveyard Freestanding PoH":
            keep = False
        if zone == "remake" and any(r["hash"] == GRAVEYARD_HASH for r in e.get("refs", [])):
            keep = False
        if not keep:
            continue
        out.append(dump(e) + ("," if comma else ""))
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write("\n".join(out))


if __name__ == "__main__":
    main(sys.argv[1])
