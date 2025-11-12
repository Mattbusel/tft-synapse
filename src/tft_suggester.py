# tft_suggester.py
from dataclasses import dataclass
from typing import Dict, List, Tuple
import yaml
import math

# --------- Load YAML configs ----------
def load_configs():
    with open("augments.yaml") as f:
        AUGMENTS = yaml.safe_load(f)
    with open("traits.yaml") as f:
        TRAITS = yaml.safe_load(f)
    with open("items.yaml") as f:
        ITEMS = yaml.safe_load(f)
    with open("config.yaml") as f:
        CONFIG = yaml.safe_load(f)
    return AUGMENTS, TRAITS, ITEMS, CONFIG

# --------- Game state model ----------
@dataclass
class State:
    stage: str                 # e.g., "3-2"
    hp: int                    # 1..100
    gold: int                  # current gold
    level: int                 # player level
    traits: Dict[str, int]     # {"Bruiser": 2, "Sorcerer": 3, ...}
    bench_parts: Dict[str, int]# {"Belt":1, "Rod":1, ...}
    taken_augments: List[str]  # already picked augments (names)

# --------- Scoring helpers ----------
def stage_urgency(stage: str) -> float:
    """Later stages raise urgency (favor immediate combat over econ). 0..1"""
    try:
        s = int(stage.split("-")[0])
    except Exception:
        s = 2
    return max(0.0, min(1.0, (s - 2) / 4.0))  # 2->0.0, 6->1.0

def hp_danger(hp: int) -> float:
    """Lower HP raises urgency. 60 HP and below ramps up to 1.0."""
    return max(0.0, min(1.0, (60 - hp) / 60.0))

def proximity_to_next_tier(traits: Dict[str, int], breakpoints: Dict[int, int], prefer: List[str]) -> float:
    """Return 0..1 bonus based on how close preferred traits are to their next breakpoint."""
    bonus = 0.0
    next_stops = sorted(breakpoints.keys())
    for t in prefer:
        if t in traits:
            cur = traits[t]
            nxt = next((bp for bp in next_stops if bp > cur), None)
            if nxt:
                dist = max(1, nxt - cur)
                bonus += 1.0 / dist  # closer => bigger
    return max(0.0, min(1.0, bonus))

def item_slam_bonus(parts: Dict[str, int], augment: str, slam_values: Dict[str, int]) -> float:
    """Reward augments that increase immediate board strength or components."""
    if augment in {"Component Grab Bag", "Portable Forge", "Pandora’s Items", "Pandoras Items", "Pandora's Items"}:
        raw = sum(parts.get(k, 0) * slam_values.get(k, 0) for k in slam_values.keys())
        return max(0.0, min(1.0, raw / 20.0))
    if augment in {"Sunfire Board", "Exiles", "Triumphant Return"}:
        raw = parts.get("Belt", 0) * 10 + parts.get("Chain", 0) * 9
        return max(0.0, min(1.0, raw / 15.0))
    return 0.0

def conflict_penalty(augment: str, taken: List[str]) -> float:
    return 1.0 if augment in taken else 0.0

def tags_to_prefer_groups(augment_tags: List[str], trait_groups: Dict[str, Dict[str, List[str]]]) -> List[str]:
    """
    Map augment tags like 'AP','AD','Tank' to the underlying trait families
    defined in traits.yaml (trait_groups.*.traits). We return a flat list of trait names.
    """
    prefer_traits: List[str] = []
    groups = trait_groups.get("trait_groups", {})
    for gname, ginfo in groups.items():
        if gname in augment_tags:
            prefer_traits.extend(ginfo.get("traits", []))
    return prefer_traits

def synergy_tag_bonus(augment_tags: List[str], prefer_traits: List[str], state_traits: Dict[str, int]) -> float:
    """
    Light synergy bump if augment tags match board identity.
    e.g., Augment has 'AP' tag and you already run Sorcerer/Invoker.
    """
    if not augment_tags:
        return 0.0
    matches = sum(1 for t in prefer_traits if t in state_traits)
    if matches == 0:
        return 0.0
    # cap to 1.0
    return max(0.0, min(1.0, matches / 2.0))  # 1 match => 0.5, 2+ => 1.0

# --------- Main scoring ----------
def score_option(augment: str, st: State, cfg, aug_db, traits_db, items_db) -> Tuple[float, Dict[str, float]]:
    base_entry = aug_db["base_scores"].get(augment, {"score": 60, "tags": []})
    base = float(base_entry.get("score", 60))
    tags = base_entry.get("tags", [])

    W = cfg["weights"]
    breakpoints = cfg["trait_breakpoints"]
    slam_values = items_db["components"]

    prefer_traits = tags_to_prefer_groups(tags, traits_db)
    f_trait = proximity_to_next_tier(st.traits, breakpoints, prefer_traits)
    f_items = item_slam_bonus(st.bench_parts, augment, slam_values)
    f_stage = stage_urgency(st.stage)
    f_hp = hp_danger(st.hp)
    f_conf = conflict_penalty(augment, st.taken_augments)
    f_syn = synergy_tag_bonus(tags, prefer_traits, st.traits)

    mult = (
        1
        + W["W_TRAIT"] * f_trait
        + W["W_ITEMS"] * f_items
        + W["W_STAGE"] * f_stage
        + W["W_HP"] * f_hp
        + W["W_SYNERGY"] * f_syn
        + W["W_CONFLICT"] * f_conf
    )
    score = base * mult

    details = {
        "base": base,
        "f_trait": f_trait,
        "f_items": f_items,
        "f_stage": f_stage,
        "f_hp": f_hp,
        "f_syn": f_syn,
        "f_conflict": f_conf,
        "multiplier": mult,
    }
    return score, details

def choose(augments: List[str], st: State):
    AUGMENTS, TRAITS, ITEMS, CONFIG = load_configs()
    scored = []
    for a in augments:
        s, d = score_option(a, st, CONFIG, AUGMENTS, TRAITS, ITEMS)
        scored.append((a, s, d))
    scored.sort(key=lambda x: x[1], reverse=True)
    return scored

# --------- Pretty print ----------
def explain_choice(scored: List[Tuple[str, float, Dict[str, float]]]) -> str:
    lines = []
    for i, (name, score, d) in enumerate(scored, 1):
        lines.append(f"{i}. {name}: {score:.1f}")
        contribs = [
            ("Trait proximity", "f_trait", "W_TRAIT"),
            ("Item slam", "f_items", "W_ITEMS"),
            ("Stage urgency", "f_stage", "W_STAGE"),
            ("HP danger", "f_hp", "W_HP"),
            ("Synergy tags", "f_syn", "W_SYNERGY"),
            ("Conflict (penalty)", "f_conflict", "W_CONFLICT"),
        ]
        # reconstruct approximate contribution summary
        lines.append(f"   base={d['base']:.1f}  x  mult={d['multiplier']:.3f}")
        lines.append("   reasons:")
        for label, key, wname in contribs:
            val = d.get(key, 0.0)
            if abs(val) < 1e-6:
                continue
            lines.append(f"     • {label}: {val:.2f}")
    return "\n".join(lines)

# --------- Demo ----------
if __name__ == "__main__":
    # Example current state; change these values to your situation
    state = State(
        stage="3-2",
        hp=52,
        gold=20,
        level=6,
        traits={"Sorcerer": 3, "Bruiser": 2},
        bench_parts={"Belt": 1, "Rod": 1, "Bow": 0, "Chain": 0, "Tear": 0, "Cloak": 0, "Glove": 0, "Sword": 0},
        taken_augments=["Jeweled Lotus"]
    )

    # The three offered augments at the pick
    offered = ["Blue Battery", "Component Grab Bag", "Sunfire Board"]

    ranked = choose(offered, state)
    print("Recommended augment order:\n")
    print(explain_choice(ranked))

