use anyhow::*;
use clap::Parser;
use serde::Deserialize;
use std::{collections::HashMap, fs};

#[derive(Parser, Debug)]
#[command(name="tft-synapse", about="TFT augment suggester (rule-based)")]
struct Args {
    /// Comma-separated augments being offered
    #[arg(long)]
    augments: String,
    /// Stage like 3-2
    #[arg(long, default_value="3-2")]
    stage: String,
    /// HP 1..100
    #[arg(long, default_value_t=60)]
    hp: i32,
    /// Traits like Sorcerer=3,Bruiser=2
    #[arg(long, default_value="")]
    traits: String,
    /// Components like Belt=1,Rod=1
    #[arg(long, default_value="")]
    parts: String,
    /// Previously taken augments (comma-separated)
    #[arg(long, default_value="")]
    taken: String,
    /// Data dir containing YAMLs
    #[arg(long, default_value=".")]
    data_dir: String,
}

#[derive(Deserialize)]
struct Augments {
    base_scores: HashMap<String, AugEntry>,
}
#[derive(Deserialize)]
struct AugEntry {
    score: f32,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct TraitsDb {
    trait_groups: HashMap<String, TraitGroup>,
}
#[derive(Deserialize)]
struct TraitGroup {
    traits: Vec<String>,
}

#[derive(Deserialize)]
struct ItemsDb {
    components: HashMap<String, i32>,
}

#[derive(Deserialize)]
struct Config {
    weights: Weights,
    trait_breakpoints: HashMap<i32, i32>,
}
#[derive(Deserialize)]
struct Weights {
    W_TRAIT: f32,
    W_ITEMS: f32,
    W_STAGE: f32,
    W_HP: f32,
    W_CONFLICT: f32,
    W_SYNERGY: f32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let aug: Augments = load_yaml(&format!("{}/augments.yaml", args.data_dir))?;
    let traits_db: TraitsDb = load_yaml(&format!("{}/traits.yaml", args.data_dir))?;
    let items_db: ItemsDb = load_yaml(&format!("{}/items.yaml", args.data_dir))?;
    let cfg: Config = load_yaml(&format!("{}/config.yaml", args.data_dir))?;

    let offered: Vec<String> = split_csv(&args.augments);
    let taken = split_csv(&args.taken);
    let traits = parse_kv(&args.traits);
    let parts = parse_kv(&args.parts);

    let mut scored = Vec::new();
    for a in offered {
        let (score, mult, detail) = score_one(
            &a, &aug, &traits_db, &items_db, &cfg,
            &args.stage, args.hp, &traits, &parts, &taken
        );
        scored.push((a, score, mult, detail));
    }
    scored.sort_by(|x,y| y.1.partial_cmp(&x.1).unwrap());

    println!("Recommended order:\n");
    for (i,(name, score, mult, d)) in scored.iter().enumerate() {
        println!("{}. {}: {:.1}", i+1, name, score);
        println!("   base={:.1} x mult={:.3}", d.base, mult);
        println!("   reasons:");
        if d.f_trait != 0.0 { println!("     • Trait proximity: {:.2}", d.f_trait); }
        if d.f_items != 0.0 { println!("     • Item slam: {:.2}", d.f_items); }
        if d.f_stage != 0.0 { println!("     • Stage urgency: {:.2}", d.f_stage); }
        if d.f_hp    != 0.0 { println!("     • HP danger: {:.2}", d.f_hp); }
        if d.f_syn   != 0.0 { println!("     • Synergy tags: {:.2}", d.f_syn); }
        if d.f_conf  != 0.0 { println!("     • Conflict (penalty): {:.2}", d.f_conf); }
        println!();
    }
    Ok(())
}

#[derive(Default)]
struct Detail { base:f32, f_trait:f32, f_items:f32, f_stage:f32, f_hp:f32, f_syn:f32, f_conf:f32 }

fn score_one(
    augment: &str,
    aug_db: &Augments,
    traits_db: &TraitsDb,
    items_db: &ItemsDb,
    cfg: &Config,
    stage: &str,
    hp: i32,
    state_traits: &HashMap<String,i32>,
    parts: &HashMap<String,i32>,
    taken: &Vec<String>,
) -> (f32, f32, Detail) {
    let mut det = Detail::default();
    let base = aug_db.base_scores.get(augment).map(|e| e.score).unwrap_or(60.0);
    det.base = base;

    let tags = aug_db.base_scores.get(augment).map(|e| e.tags.clone()).unwrap_or_default();
    let prefer_traits = tags_to_prefer_traits(&tags, &traits_db.trait_groups);

    det.f_trait = proximity_to_next_tier(state_traits, &cfg.trait_breakpoints, &prefer_traits);
    det.f_items = item_slam_bonus(parts, augment, &items_db.components);
    det.f_stage = stage_urgency(stage);
    det.f_hp    = hp_danger(hp);
    det.f_conf  = if taken.iter().any(|t| t==augment) { 1.0 } else { 0.0 };
    det.f_syn   = synergy_tag_bonus(&prefer_traits, state_traits);

    let w = &cfg.weights;
    let mult = 1.0
        + w.W_TRAIT * det.f_trait
        + w.W_ITEMS * det.f_items
        + w.W_STAGE * det.f_stage
        + w.W_HP    * det.f_hp
        + w.W_SYNERGY * det.f_syn
        + w.W_CONFLICT * det.f_conf;

    (base * mult, mult, det)
}

fn tags_to_prefer_traits(tags:&Vec<String>, groups:&HashMap<String, TraitGroup>) -> Vec<String> {
    let mut out = Vec::new();
    for (gname, g) in groups {
        if tags.iter().any(|t| t==gname) {
            out.extend(g.traits.iter().cloned());
        }
    }
    out
}

fn proximity_to_next_tier(traits:&HashMap<String,i32>, breaks:&HashMap<i32,i32>, prefer:&Vec<String>) -> f32 {
    let mut bonus = 0.0;
    let mut stops: Vec<i32> = breaks.keys().cloned().collect();
    stops.sort_unstable();
    for t in prefer {
        if let Some(cur) = traits.get(t) {
            if let Some(nxt) = stops.iter().copied().find(|bp| bp > cur.to_owned()) {
                let dist = (nxt - *cur).max(1) as f32;
                bonus += 1.0 / dist;
            }
        }
    }
    bonus.clamp(0.0, 1.0)
}

fn item_slam_bonus(parts:&HashMap<String,i32>, augment:&str, slam:&HashMap<String,i32>) -> f32 {
    let aug = augment.to_lowercase();
    if ["component grab bag","portable forge","pandora’s items","pandoras items","pandora's items"].iter().any(|a| aug==*a) {
        let raw:i32 = slam.iter().map(|(k,v)| parts.get(k).unwrap_or(&0) * v).sum();
        return (raw as f32 / 20.0).clamp(0.0, 1.0)
    }
    if ["sunfire board","exiles","triumphant return"].iter().any(|a| aug==*a) {
        let raw = parts.get("Belt").unwrap_or(&0)*10 + parts.get("Chain").unwrap_or(&0)*9;
        return (raw as f32 / 15.0).clamp(0.0, 1.0)
    }
    0.0
}

fn stage_urgency(stage:&str) -> f32 {
    let s = stage.split('-').next().and_then(|x| x.parse::<i32>().ok()).unwrap_or(2);
    (((s - 2) as f32) / 4.0).clamp(0.0, 1.0)
}

fn hp_danger(hp:i32) -> f32 { (((60 - hp) as f32) / 60.0).clamp(0.0, 1.0) }

fn split_csv(s:&str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}
fn parse_kv(s:&str) -> HashMap<String,i32> {
    let mut m = HashMap::new();
    for part in s.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
        if let Some((k,v)) = part.split_once('=') {
            if let Ok(n) = v.parse::<i32>() { m.insert(k.to_string(), n); }
        }
    }
    m
}

fn load_yaml<T: for<'de> serde::Deserialize<'de>>(path:String) -> Result<T> {
    let s = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path))?;
    let val = serde_yaml::from_str::<T>(&s)
        .with_context(|| format!("Failed to parse YAML {}", path))?;
    Ok(val)
}
