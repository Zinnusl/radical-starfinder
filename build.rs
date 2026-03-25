use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct VocabRow {
    hanzi: String,
    pinyin: String,
    meaning: String,
    hsk: u8,
}

fn main() {
    println!("cargo:rerun-if-changed=data/hsk30.csv");
    println!("cargo:rerun-if-changed=data/cedict_ts.u8");
    println!("cargo:rerun-if-changed=data/ids.txt");
    println!("cargo:rerun-if-changed=data/unique_chars.txt");
    println!("cargo:rerun-if-changed=data/dialogues/starmap");
    println!("cargo:rerun-if-changed=data/dialogues/exploration");

    if let Err(err) = generate_vocab_data() {
        println!("cargo:warning=build.rs vocab generation warning: {}", err);
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let out_path = Path::new(&out_dir).join("vocab_data.rs");
            let _ = fs::write(out_path, "pub static VOCAB: &[VocabEntry] = &[];\n");
        }
    }

    if let Err(err) = generate_decomposition_data() {
        println!(
            "cargo:warning=build.rs decomposition generation warning: {}",
            err
        );
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let out_path = Path::new(&out_dir).join("decomposition_data.rs");
            let _ = fs::write(
                out_path,
                "pub fn get_components(_hanzi: &str) -> Vec<&'static str> { vec![] }\n",
            );
        }
    }

    if let Err(err) = generate_dialogue_data() {
        println!(
            "cargo:warning=build.rs dialogue generation warning: {}",
            err
        );
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let out_path = Path::new(&out_dir).join("dialogue_data.rs");
            let _ = fs::write(
                out_path,
                concat!(
                    "pub static ALL_STARMAP_EVENTS: &[super::events::SpaceEvent] = &[];\n",
                    "pub static ALL_DUNGEON_DIALOGUES: &[DungeonDialogue] = &[];\n",
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Ink dialogue parser
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct InkChoice {
    text: String,
    chinese_hint: String,
    outcome: String,
    requires: Option<String>,
}

#[derive(Debug)]
struct InkDialogue {
    id: usize,
    title: String,
    chinese_title: String,
    description: String,
    category: String,
    mode: String,
    choices: Vec<InkChoice>,
}

fn collect_ink_files(dir: &str) -> Vec<std::path::PathBuf> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ink") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn parse_ink_file(path: &Path) -> Result<Vec<InkDialogue>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    let mut dialogues: Vec<InkDialogue> = Vec::new();
    let mut current: Option<InkDialogue> = None;
    let mut in_description = false;
    let mut desc_buf = String::new();
    let mut choices: Vec<InkChoice> = Vec::new();
    // Partial choice being built
    let mut cur_choice_text: Option<String> = None;
    let mut cur_choice_requires: Option<String> = None;
    let mut cur_choice_outcome: Option<String> = None;

    let flush_choice = |choices: &mut Vec<InkChoice>,
                        text: &mut Option<String>,
                        requires: &mut Option<String>,
                        outcome: &mut Option<String>,
                        hint: &str| {
        if let Some(t) = text.take() {
            choices.push(InkChoice {
                text: t,
                chinese_hint: hint.to_string(),
                outcome: outcome.take().unwrap_or_else(|| "nothing".to_string()),
                requires: requires.take(),
            });
        } else {
            *requires = None;
            *outcome = None;
        }
    };

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // === EVENT_NAME === starts a new dialogue
        if line.starts_with("===") && line.ends_with("===") {
            // Flush previous choice (with empty hint)
            flush_choice(
                &mut choices,
                &mut cur_choice_text,
                &mut cur_choice_requires,
                &mut cur_choice_outcome,
                "",
            );
            // Flush previous dialogue
            if let Some(mut d) = current.take() {
                d.description = desc_buf.trim().to_string();
                d.choices = std::mem::take(&mut choices);
                dialogues.push(d);
            }
            desc_buf.clear();
            in_description = false;
            current = Some(InkDialogue {
                id: 0,
                title: String::new(),
                chinese_title: String::new(),
                description: String::new(),
                category: String::new(),
                mode: String::new(),
                choices: Vec::new(),
            });
            continue;
        }

        // Skip if no dialogue started yet
        if current.is_none() {
            continue;
        }

        // Metadata lines
        if line.starts_with("# ") {
            let meta = &line[2..];
            if let Some((key, val)) = meta.split_once(':') {
                let key = key.trim();
                let val = val.trim();
                let d = current.as_mut().unwrap();
                match key {
                    "id" => d.id = val.parse().unwrap_or(0),
                    "title" => d.title = val.to_string(),
                    "chinese_title" => d.chinese_title = val.to_string(),
                    "category" => d.category = val.to_string(),
                    "mode" => d.mode = val.to_string(),
                    _ => {}
                }
            }
            continue;
        }

        // Choice line: * [text] {requires: ...}
        if line.starts_with("* [") {
            // Flush previous choice (with empty hint since we haven't seen the hint line yet)
            flush_choice(
                &mut choices,
                &mut cur_choice_text,
                &mut cur_choice_requires,
                &mut cur_choice_outcome,
                "",
            );

            in_description = false;

            // Parse choice text and optional requirement
            let after_star = &line[2..]; // "[text] {requires: ...}" or "[text]"
            if let Some(bracket_end) = after_star.find(']') {
                cur_choice_text = Some(after_star[1..bracket_end].trim().to_string());

                let rest = after_star[bracket_end + 1..].trim();
                if rest.starts_with('{') {
                    if let Some(brace_end) = rest.find('}') {
                        let req_str = rest[1..brace_end].trim();
                        if let Some(req_val) = req_str.strip_prefix("requires:") {
                            cur_choice_requires = Some(req_val.trim().to_string());
                        }
                    }
                }
            }
            continue;
        }

        // Outcome line: ~ outcome(args)
        if line.starts_with("~ ") {
            in_description = false;
            cur_choice_outcome = Some(line[2..].trim().to_string());
            continue;
        }

        // If we have a pending choice with an outcome, the next non-empty line is the chinese hint
        if cur_choice_text.is_some() && cur_choice_outcome.is_some() && !line.is_empty() {
            let hint = line.to_string();
            flush_choice(
                &mut choices,
                &mut cur_choice_text,
                &mut cur_choice_requires,
                &mut cur_choice_outcome,
                &hint,
            );
            continue;
        }

        // Description text (between metadata and first choice)
        if current.as_ref().map_or(false, |d| !d.category.is_empty()) && cur_choice_text.is_none()
        {
            if !line.is_empty() || !desc_buf.is_empty() {
                in_description = true;
            }
            if in_description {
                if !desc_buf.is_empty() {
                    desc_buf.push(' ');
                }
                desc_buf.push_str(line);
            }
        }
    }

    // Flush last choice
    flush_choice(
        &mut choices,
        &mut cur_choice_text,
        &mut cur_choice_requires,
        &mut cur_choice_outcome,
        "",
    );

    // Flush last dialogue
    if let Some(mut d) = current.take() {
        d.description = desc_buf.trim().to_string();
        d.choices = std::mem::take(&mut choices);
        dialogues.push(d);
    }

    Ok(dialogues)
}

fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn format_starmap_outcome(raw: &str) -> String {
    let s = raw.trim();
    // Match outcome command and arguments
    if s == "nothing" {
        return "super::events::EventOutcome::Nothing".to_string();
    }
    if s == "gain_crew_member" {
        return "super::events::EventOutcome::GainCrewMember".to_string();
    }
    if s == "lose_crew_member" {
        return "super::events::EventOutcome::LoseCrewMember".to_string();
    }

    // Two-arg forms: fuel_and_credits(N, M), hull_and_fuel(N, M), combat_reward(N, M)
    if let Some(inner) = extract_args(s, "fuel_and_credits") {
        if let Some((a, b)) = split_two_args(&inner) {
            return format!("super::events::EventOutcome::FuelAndCredits({}, {})", a, b);
        }
    }
    if let Some(inner) = extract_args(s, "hull_and_fuel") {
        if let Some((a, b)) = split_two_args(&inner) {
            return format!("super::events::EventOutcome::HullAndFuel({}, {})", a, b);
        }
    }
    if let Some(inner) = extract_args(s, "combat_reward") {
        if let Some((a, b)) = split_two_args(&inner) {
            return format!("super::events::EventOutcome::CombatReward({}, {})", a, b);
        }
    }

    // String-arg forms
    if let Some(inner) = extract_args(s, "gain_radical") {
        let cleaned = inner.trim().trim_matches('"');
        return format!(
            "super::events::EventOutcome::GainRadical(\"{}\")",
            escape_str(cleaned)
        );
    }
    if let Some(inner) = extract_args(s, "gain_item") {
        let cleaned = inner.trim().trim_matches('"');
        return format!(
            "super::events::EventOutcome::GainItem(\"{}\")",
            escape_str(cleaned)
        );
    }

    // Single numeric arg forms
    let single_mappings: &[(&str, &str)] = &[
        ("gain_fuel", "GainFuel"),
        ("lose_fuel", "LoseFuel"),
        ("gain_credits", "GainCredits"),
        ("lose_credits", "LoseCredits"),
        ("gain_hull", "GainHull"),
        ("lose_hull", "LoseHull"),
        ("heal_crew", "HealCrew"),
        ("damage_crew", "DamageCrew"),
        ("repair_ship", "RepairShip"),
        ("gain_scrap", "GainScrap"),
        ("shield_damage", "ShieldDamage"),
        ("start_combat", "StartCombat"),
    ];

    for (cmd, variant) in single_mappings {
        if let Some(inner) = extract_args(s, cmd) {
            let n = inner.trim();
            n.parse::<i32>().unwrap_or_else(|_| panic!("expected integer argument for {}, got '{}'", cmd, n));
            return format!("super::events::EventOutcome::{}({})", variant, n);
        }
    }

    // Fallback
    format!("super::events::EventOutcome::Nothing /* unrecognised: {} */", escape_str(s))
}

fn format_dungeon_outcome(raw: &str) -> String {
    let s = raw.trim();
    if s == "nothing" {
        return "DungeonOutcome::Nothing".to_string();
    }
    if s == "gain_equipment" {
        return "DungeonOutcome::GainEquipment".to_string();
    }
    if s == "start_fight" {
        return "DungeonOutcome::StartFight".to_string();
    }
    if s == "gain_crew_member" {
        return "DungeonOutcome::GainCrewMember".to_string();
    }

    if let Some(inner) = extract_args(s, "gain_radical") {
        let cleaned = inner.trim().trim_matches('"');
        return format!("DungeonOutcome::GainRadical(\"{}\")", escape_str(cleaned));
    }
    if let Some(inner) = extract_args(s, "gain_item") {
        let cleaned = inner.trim().trim_matches('"');
        return format!("DungeonOutcome::GainItem(\"{}\")", escape_str(cleaned));
    }

    let single_mappings: &[(&str, &str)] = &[
        ("heal", "Heal"),
        ("damage", "Damage"),
        ("gain_gold", "GainGold"),
        ("lose_gold", "LoseGold"),
        ("gain_xp", "GainXp"),
        ("gain_credits", "GainCredits"),
        ("lose_credits", "LoseCredits"),
    ];

    for (cmd, variant) in single_mappings {
        if let Some(inner) = extract_args(s, cmd) {
            let n = inner.trim();
            n.parse::<i32>().unwrap_or_else(|_| panic!("expected integer argument for {}, got '{}'", cmd, n));
            return format!("DungeonOutcome::{}({})", variant, n);
        }
    }

    format!("DungeonOutcome::Nothing /* unrecognised: {} */", escape_str(s))
}

fn format_starmap_requirement(raw: &str) -> String {
    let s = raw.trim();
    // fuel >= N
    if let Some(rest) = s.strip_prefix("fuel") {
        let rest = rest.trim().strip_prefix(">=").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(super::events::EventRequirement::HasFuel({}))", n);
    }
    // credits >= N
    if let Some(rest) = s.strip_prefix("credits") {
        let rest = rest.trim().strip_prefix(">=").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(super::events::EventRequirement::HasCredits({}))", n);
    }
    // crew_role == N
    if let Some(rest) = s.strip_prefix("crew_role") {
        let rest = rest.trim().strip_prefix("==").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(super::events::EventRequirement::HasCrewRole({}))", n);
    }
    // class == N
    if let Some(rest) = s.strip_prefix("class") {
        let rest = rest.trim().strip_prefix("==").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(super::events::EventRequirement::HasClass({}))", n);
    }
    // radical == "X"
    if let Some(rest) = s.strip_prefix("radical") {
        let rest = rest.trim().strip_prefix("==").unwrap_or(rest.trim());
        let cleaned = rest.trim().trim_matches('"');
        return format!(
            "Some(super::events::EventRequirement::HasRadical(\"{}\"))",
            escape_str(cleaned)
        );
    }
    "None".to_string()
}

fn format_dungeon_requirement(raw: &str) -> String {
    let s = raw.trim();
    if let Some(rest) = s.strip_prefix("gold") {
        let rest = rest.trim().strip_prefix(">=").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(DungeonRequirement::HasGold({}))", n);
    }
    if let Some(rest) = s.strip_prefix("hp") {
        let rest = rest.trim().strip_prefix(">=").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(DungeonRequirement::HasHp({}))", n);
    }
    if let Some(rest) = s.strip_prefix("class") {
        let rest = rest.trim().strip_prefix("==").unwrap_or(rest.trim());
        let n = rest.trim();
        return format!("Some(DungeonRequirement::HasClass({}))", n);
    }
    if let Some(rest) = s.strip_prefix("radical") {
        let rest = rest.trim().strip_prefix("==").unwrap_or(rest.trim());
        let cleaned = rest.trim().trim_matches('"');
        return format!(
            "Some(DungeonRequirement::HasRadical(\"{}\"))",
            escape_str(cleaned)
        );
    }
    "None".to_string()
}

fn extract_args(s: &str, prefix: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with(prefix) {
        return None;
    }
    let rest = &s[prefix.len()..];
    let rest = rest.trim();
    if rest.starts_with('(') {
        if let Some(end) = rest.find(')') {
            return Some(rest[1..end].to_string());
        }
    }
    None
}

fn split_two_args(s: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = s.splitn(2, ',').collect();
    if parts.len() == 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
    }
}

fn format_starmap_category(cat: &str) -> String {
    let variant = match cat.trim() {
        "DistressSignal" => "DistressSignal",
        "PirateEncounter" => "PirateEncounter",
        "Trading" => "Trading",
        "Discovery" => "Discovery",
        "AnomalyEncounter" => "AnomalyEncounter",
        "CrewEvent" => "CrewEvent",
        "AlienContact" => "AlienContact",
        "HazardEvent" => "HazardEvent",
        "AncientRuins" => "AncientRuins",
        "LanguageChallenge" => "LanguageChallenge",
        other => other,
    };
    format!("super::events::EventCategory::{}", variant)
}

fn format_dungeon_category(cat: &str) -> String {
    let variant = match cat.trim() {
        "Discovery" => "Discovery",
        "Trader" => "Trader",
        "Alien" => "Alien",
        "Hazard" => "Hazard",
        "Crew" => "Crew",
        "Puzzle" => "Puzzle",
        "Lore" => "Lore",
        "Shrine" => "Shrine",
        "Wreckage" => "Wreckage",
        "Terminal" => "Terminal",
        "Creature" => "Creature",
        "Anomaly" => "Anomaly",
        other => other,
    };
    format!("DungeonCategory::{}", variant)
}

fn generate_dialogue_data() -> Result<(), String> {
    let out_dir = env::var("OUT_DIR").map_err(|e| format!("OUT_DIR not set: {e}"))?;
    let out_path = Path::new(&out_dir).join("dialogue_data.rs");

    let starmap_files = collect_ink_files("data/dialogues/starmap");
    let exploration_files = collect_ink_files("data/dialogues/exploration");

    let mut starmap_dialogues: Vec<InkDialogue> = Vec::new();
    let mut dungeon_dialogues: Vec<InkDialogue> = Vec::new();

    for f in &starmap_files {
        println!("cargo:rerun-if-changed={}", f.display());
        match parse_ink_file(f) {
            Ok(ds) => {
                for d in ds {
                    starmap_dialogues.push(d);
                }
            }
            Err(e) => panic!("ink parse error in {}: {}", f.display(), e),
        }
    }

    for f in &exploration_files {
        println!("cargo:rerun-if-changed={}", f.display());
        match parse_ink_file(f) {
            Ok(ds) => {
                for d in ds {
                    dungeon_dialogues.push(d);
                }
            }
            Err(e) => panic!("ink parse error in {}: {}", f.display(), e),
        }
    }

    // Assign sequential IDs
    for (i, d) in starmap_dialogues.iter_mut().enumerate() {
        d.id = i;
    }
    for (i, d) in dungeon_dialogues.iter_mut().enumerate() {
        d.id = i;
    }

    let mut out = String::new();
    out.push_str("// Auto-generated by build.rs from data/dialogues/**/*.ink\n");
    out.push_str("// Do not edit manually.\n\n");

    // ── Starmap events ──────────────────────────────────────────────────

    // Emit choice arrays first
    for (idx, d) in starmap_dialogues.iter().enumerate() {
        out.push_str(&format!(
            "static STARMAP_CHOICES_{}: &[super::events::EventChoice] = &[\n",
            idx
        ));
        for c in &d.choices {
            let outcome = format_starmap_outcome(&c.outcome);
            let requires = match &c.requires {
                Some(r) => format_starmap_requirement(r),
                None => "None".to_string(),
            };
            out.push_str(&format!(
                "    super::events::EventChoice {{\n\
                 \x20       text: \"{}\",\n\
                 \x20       chinese_hint: \"{}\",\n\
                 \x20       outcome: {},\n\
                 \x20       requires: {},\n\
                 \x20   }},\n",
                escape_str(&c.text),
                escape_str(&c.chinese_hint),
                outcome,
                requires,
            ));
        }
        out.push_str("];\n\n");
    }

    // Emit the array
    out.push_str(&format!(
        "pub static ALL_STARMAP_EVENTS: &[super::events::SpaceEvent] = &[\n"
    ));
    for (idx, d) in starmap_dialogues.iter().enumerate() {
        out.push_str(&format!(
            "    super::events::SpaceEvent {{\n\
             \x20       id: {},\n\
             \x20       title: \"{}\",\n\
             \x20       chinese_title: \"{}\",\n\
             \x20       description: \"{}\",\n\
             \x20       choices: STARMAP_CHOICES_{},\n\
             \x20       category: {},\n\
             \x20   }},\n",
            d.id,
            escape_str(&d.title),
            escape_str(&d.chinese_title),
            escape_str(&d.description),
            idx,
            format_starmap_category(&d.category),
        ));
    }
    out.push_str("];\n\n");

    // ── Dungeon dialogues ───────────────────────────────────────────────

    for (idx, d) in dungeon_dialogues.iter().enumerate() {
        out.push_str(&format!(
            "static DUNGEON_CHOICES_{}: &[DungeonChoice] = &[\n",
            idx
        ));
        for c in &d.choices {
            let outcome = format_dungeon_outcome(&c.outcome);
            let requires = match &c.requires {
                Some(r) => format_dungeon_requirement(r),
                None => "None".to_string(),
            };
            out.push_str(&format!(
                "    DungeonChoice {{\n\
                 \x20       text: \"{}\",\n\
                 \x20       chinese_hint: \"{}\",\n\
                 \x20       outcome: {},\n\
                 \x20       requires: {},\n\
                 \x20   }},\n",
                escape_str(&c.text),
                escape_str(&c.chinese_hint),
                outcome,
                requires,
            ));
        }
        out.push_str("];\n\n");
    }

    out.push_str("pub static ALL_DUNGEON_DIALOGUES: &[DungeonDialogue] = &[\n");
    for (idx, d) in dungeon_dialogues.iter().enumerate() {
        out.push_str(&format!(
            "    DungeonDialogue {{\n\
             \x20       id: {},\n\
             \x20       title: \"{}\",\n\
             \x20       chinese_title: \"{}\",\n\
             \x20       description: \"{}\",\n\
             \x20       choices: DUNGEON_CHOICES_{},\n\
             \x20       category: {},\n\
             \x20   }},\n",
            d.id,
            escape_str(&d.title),
            escape_str(&d.chinese_title),
            escape_str(&d.description),
            idx,
            format_dungeon_category(&d.category),
        ));
    }
    out.push_str("];\n");

    fs::write(&out_path, out).map_err(|e| format!("failed to write dialogue_data.rs: {e}"))?;
    Ok(())
}

fn generate_vocab_data() -> Result<(), String> {
    let cedict_map = load_cedict_map("data/cedict_ts.u8").unwrap_or_default();
    let csv = fs::read_to_string("data/hsk30.csv")
        .map_err(|e| format!("failed to read data/hsk30.csv: {e}"))?;

    let mut rows: Vec<VocabRow> = Vec::new();
    let mut seen_hanzi: HashSet<String> = HashSet::new();

    for (line_idx, line) in csv.lines().enumerate() {
        if line_idx == 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        let cols = parse_csv_line(line);
        if cols.len() < 10 {
            continue;
        }

        let hanzi = cols.get(1).map(|s| s.trim()).unwrap_or("");
        if hanzi.is_empty() || !seen_hanzi.insert(hanzi.to_string()) {
            continue;
        }

        let pinyin_col = cols.get(3).map(|s| s.trim()).unwrap_or("");
        let pos_col = cols.get(4).map(|s| s.trim()).unwrap_or("");
        let level_col = cols.get(5).map(|s| s.trim()).unwrap_or("");
        let cedict_col = cols.get(9).map(|s| s.trim()).unwrap_or("");

        let hsk = map_hsk_level(level_col);
        // Convert the CSV diacritic pinyin for matching against CEDICT entries
        let csv_numbered = diacritic_pinyin_to_numbered(pinyin_col);

        let (dict_pinyin, dict_meaning) = if let Some(entries) = cedict_map.get(hanzi) {
            pick_best_cedict_entry(entries, &csv_numbered)
        } else {
            (String::new(), String::new())
        };

        let pinyin = if !dict_pinyin.is_empty() {
            dict_pinyin
        } else if let Some(from_ref) = extract_bracket_pinyin(cedict_col) {
            from_ref
        } else {
            csv_numbered.clone()
        };

        let meaning = if !dict_meaning.is_empty() {
            dict_meaning
        } else if !pos_col.is_empty() {
            pos_col.to_string()
        } else {
            String::new()
        };

        let (pinyin, meaning) = apply_compat_overrides(hanzi, pinyin, meaning);

        rows.push(VocabRow {
            hanzi: hanzi.to_string(),
            pinyin,
            meaning,
            hsk,
        });
    }

    let mut out = String::new();
    out.push_str("// @generated by build.rs; do not edit by hand.\n");
    out.push_str("pub static VOCAB: &[VocabEntry] = &[\n");
    for row in &rows {
        out.push_str("    VocabEntry { ");
        out.push_str("hanzi: \"");
        out.push_str(&escape_rust_string(&row.hanzi));
        out.push_str("\", ");
        out.push_str("pinyin: \"");
        out.push_str(&escape_rust_string(&row.pinyin));
        out.push_str("\", ");
        out.push_str("meaning: \"");
        out.push_str(&escape_rust_string(&row.meaning));
        out.push_str("\", ");
        out.push_str("hsk: ");
        out.push_str(&row.hsk.to_string());
        out.push_str(", ");
        out.push_str("example: \"\" },\n");
    }
    out.push_str("];\n");

    let out_dir = env::var("OUT_DIR").map_err(|e| format!("OUT_DIR not set: {e}"))?;
    let out_path = Path::new(&out_dir).join("vocab_data.rs");
    fs::write(out_path, out)
        .map_err(|e| format!("failed to write generated vocab_data.rs: {e}"))?;

    Ok(())
}

fn load_cedict_map(path: &str) -> Result<HashMap<String, Vec<(String, String)>>, String> {
    let content = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Ok(HashMap::new()),
    };

    let mut map: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((simplified, pinyin, meaning)) = parse_cedict_line(line) {
            map.entry(simplified).or_default().push((pinyin, meaning));
        }
    }
    Ok(map)
}

fn parse_cedict_line(line: &str) -> Option<(String, String, String)> {
    let first_space = line.find(' ')?;
    let second_space_rel = line[first_space + 1..].find(' ')?;
    let second_space = first_space + 1 + second_space_rel;

    let simplified = line[first_space + 1..second_space].trim();
    if simplified.is_empty() {
        return None;
    }

    let bracket_start = line[second_space + 1..].find('[')? + second_space + 1;
    let bracket_end = line[bracket_start + 1..].find(']')? + bracket_start + 1;

    let pinyin_spaced = line[bracket_start + 1..bracket_end].trim();
    let pinyin = normalize_numbered_pinyin(pinyin_spaced);

    let defs_part = line[bracket_end + 1..].trim();
    let meaning = first_real_definition(defs_part).unwrap_or_default();

    Some((simplified.to_string(), pinyin, meaning))
}

fn is_bad_meaning(meaning: &str) -> bool {
    let m = meaning.to_lowercase();
    m.starts_with("surname ")
        || m.starts_with("old variant of")
        || m.starts_with("variant of")
        || m.starts_with("see ")
        || m.starts_with("cl:")
}

fn looks_like_proper_noun_only(meaning: &str) -> bool {
    let trimmed = meaning.trim();
    if trimmed.is_empty() {
        return false;
    }
    let first_char = trimmed.chars().next().unwrap_or('a');
    if !first_char.is_ascii_uppercase() {
        return false;
    }
    !trimmed.contains(';') && !trimmed.contains("to ") && trimmed.split_whitespace().count() <= 3
}

fn pick_best_cedict_entry(entries: &[(String, String)], csv_pinyin: &str) -> (String, String) {
    let mut best: Option<&(String, String)> = None;
    let mut best_score: i32 = -1;

    for entry in entries {
        let mut score: i32 = 0;
        if !is_bad_meaning(&entry.1) {
            score += 10;
        }
        if entry.0 == csv_pinyin {
            score += 5;
        }
        if looks_like_proper_noun_only(&entry.1) {
            score -= 8;
        }
        if entry.1.contains(';') || entry.1.contains("to ") {
            score += 3;
        }
        if entry.1.len() > 2 {
            score += 1;
        }
        if score > best_score {
            best_score = score;
            best = Some(entry);
        }
    }

    match best {
        Some(e) => (e.0.clone(), e.1.clone()),
        None => (String::new(), String::new()),
    }
}

fn first_real_definition(defs_part: &str) -> Option<String> {
    let start = defs_part.find('/')?;
    let tail = &defs_part[start + 1..];
    let defs = tail.split('/').map(str::trim).filter(|d| !d.is_empty());

    let mut fallback: Option<String> = None;
    for d in defs {
        if fallback.is_none() {
            fallback = Some(d.to_string());
        }
        if d.starts_with("CL:") {
            continue;
        }
        if d.starts_with("variant of") || d.starts_with("old variant of") || d.starts_with("see ") {
            continue;
        }
        return Some(d.to_string());
    }
    fallback
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().trim_end_matches('\r').to_string());
    fields
}

fn map_hsk_level(level: &str) -> u8 {
    match level.trim() {
        "1" => 1,
        "2" => 2,
        "3" => 3,
        "4" => 4,
        "5" => 5,
        "6" => 6,
        "7-9" => 7,
        _ => 7,
    }
}

fn extract_bracket_pinyin(cedict_ref: &str) -> Option<String> {
    let start = cedict_ref.find('[')?;
    let end = cedict_ref[start + 1..].find(']')? + start + 1;
    let inner = cedict_ref[start + 1..end].trim();
    if inner.is_empty() {
        return None;
    }
    Some(normalize_numbered_pinyin(inner))
}

fn normalize_numbered_pinyin(pinyin: &str) -> String {
    pinyin
        .chars()
        .filter_map(|ch| {
            if ch.is_whitespace()
                || ch == '·'
                || ch == '•'
                || ch == '‧'
                || ch == '-'
                || ch == '–'
                || ch == '—'
                || ch == '\''
                || ch == ':'
            {
                None
            } else if ch == 'ü' || ch == 'Ü' {
                Some('v')
            } else {
                Some(ch.to_ascii_lowercase())
            }
        })
        .collect()
}

fn diacritic_pinyin_to_numbered(input: &str) -> String {
    input
        .split_whitespace()
        .filter_map(convert_token)
        .collect::<Vec<_>>()
        .join("")
}

/// Map a diacritic character to (base_ascii, optional_tone_digit).
fn map_diacritic(ch: char) -> Option<(char, Option<char>)> {
    let mapped = match ch {
        'ā' => ('a', Some('1')),
        'ē' => ('e', Some('1')),
        'ī' => ('i', Some('1')),
        'ō' => ('o', Some('1')),
        'ū' => ('u', Some('1')),
        'ǖ' => ('v', Some('1')),
        'á' => ('a', Some('2')),
        'é' => ('e', Some('2')),
        'í' => ('i', Some('2')),
        'ó' => ('o', Some('2')),
        'ú' => ('u', Some('2')),
        'ǘ' => ('v', Some('2')),
        'ǎ' => ('a', Some('3')),
        'ě' => ('e', Some('3')),
        'ǐ' => ('i', Some('3')),
        'ǒ' => ('o', Some('3')),
        'ǔ' => ('u', Some('3')),
        'ǚ' => ('v', Some('3')),
        'à' => ('a', Some('4')),
        'è' => ('e', Some('4')),
        'ì' => ('i', Some('4')),
        'ò' => ('o', Some('4')),
        'ù' => ('u', Some('4')),
        'ǜ' => ('v', Some('4')),
        'ü' | 'Ü' => ('v', None),
        'A'..='Z' | 'a'..='z' => (ch.to_ascii_lowercase(), None),
        _ => return None,
    };
    Some(mapped)
}

/// Convert a whitespace-separated token (which may contain multiple
/// concatenated pinyin syllables) into numbered pinyin.
/// E.g. "péngyoumen" → "peng2you5men5", "àihào" → "ai4hao4".
fn convert_token(raw: &str) -> Option<String> {
    // Pass 1: strip diacritics → plain ASCII, record tone for each position.
    let mut ascii = String::new();
    let mut tones: Vec<Option<char>> = Vec::new();
    let mut explicit_tone_digit: Option<char> = None;

    for ch in raw.chars() {
        if ('1'..='5').contains(&ch) {
            explicit_tone_digit = Some(ch);
            continue;
        }
        if let Some((base, tone)) = map_diacritic(ch) {
            tones.push(tone);
            ascii.push(base);
        }
    }

    if ascii.is_empty() {
        return None;
    }

    // If the token is a single syllable (has explicit digit or only one
    // syllable after splitting), use the old direct path for robustness.
    if let Some(digit) = explicit_tone_digit {
        return Some(format!("{}{}", ascii, digit));
    }

    // Pass 2: greedy longest-match split into valid pinyin syllables.
    let syllables = split_pinyin_syllables(&ascii);

    // Pass 3: assign tones to each syllable.
    let mut result = String::new();
    let mut pos = 0;
    for syl in &syllables {
        let syl_len = syl.len();
        let mut tone = None;
        for i in pos..pos + syl_len {
            if let Some(Some(t)) = tones.get(i) {
                tone = Some(*t);
            }
        }
        result.push_str(syl);
        result.push(tone.unwrap_or('5'));
        pos += syl_len;
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Valid pinyin syllables (without tones) used for greedy splitting.
/// Listed longest-first so longest match wins.
fn pinyin_syllable_table() -> &'static [&'static str] {
    &[
        // 6-letter
        "zhuang", "shuang", "chuang", // 5-letter
        "zhuan", "zheng", "zhang", "zhuai", "zhong", "zhuan", "shuan", "sheng", "shang", "shuai",
        "shuai", "chuan", "cheng", "chang", "chuai", "chong", "guang", "huang", "kuang", "niang",
        "qiang", "xiang", "jiong", "qiong", "xiong", "liang", // 4-letter
        "zhan", "zhao", "zhei", "zhen", "zhai", "zhao", "zhou", "zhua", "shan", "shao", "shei",
        "shen", "shou", "shua", "shuo", "shai", "shao", "chan", "chao", "chen", "chou", "chua",
        "chui", "chun", "chuo", "chai", "bang", "beng", "bing", "biao", "cang", "ceng", "cong",
        "cuan", "dang", "deng", "ding", "dong", "dian", "diao", "duan", "fang", "feng", "fiao",
        "gang", "geng", "gong", "guan", "guai", "giao", "hang", "heng", "hong", "huan", "huai",
        "jian", "jiao", "jing", "juan", "jiang", "jiue", "kang", "keng", "kong", "kuan", "kuai",
        "lang", "leng", "ling", "long", "lian", "liao", "luan", "mang", "meng", "ming", "mian",
        "miao", "nang", "neng", "ning", "nong", "nian", "niao", "nuan", "pang", "peng", "ping",
        "pian", "piao", "qian", "qiao", "qing", "quan", "rang", "reng", "rong", "ruan", "sang",
        "seng", "song", "suan", "tang", "teng", "ting", "tong", "tian", "tiao", "tuan", "wang",
        "weng", "xian", "xiao", "xing", "xuan", "yang", "ying", "yong", "yuan", "zang", "zeng",
        "zong", "zuan", // 3-letter
        "zhi", "zhu", "zhe", "zha", "zhu", "shi", "shu", "she", "sha", "chi", "chu", "che", "cha",
        "bai", "ban", "bao", "bei", "ben", "bin", "cai", "can", "cao", "cei", "cen", "cou", "cui",
        "cun", "cuo", "dai", "dan", "dao", "dei", "den", "dia", "die", "diu", "dou", "dui", "dun",
        "duo", "fan", "fei", "fen", "fou", "gai", "gan", "gao", "gei", "gen", "gou", "gua", "gui",
        "gun", "guo", "hai", "han", "hao", "hei", "hen", "hou", "hua", "hui", "hun", "huo", "jia",
        "jie", "jin", "jiu", "jue", "jun", "kai", "kan", "kao", "kei", "ken", "kou", "kua", "kui",
        "kun", "kuo", "lai", "lan", "lao", "lei", "lia", "lie", "lin", "liu", "lou", "lun", "luo",
        "lve", "mai", "man", "mao", "mei", "men", "mie", "min", "miu", "mou", "nai", "nan", "nao",
        "nei", "nen", "nie", "nin", "niu", "nou", "nun", "nuo", "nve", "pai", "pan", "pao", "pei",
        "pen", "pie", "pin", "pou", "qia", "qie", "qin", "qiu", "que", "qun", "ran", "rao", "ren",
        "rou", "rua", "rui", "run", "ruo", "sai", "san", "sao", "sei", "sen", "sou", "sui", "sun",
        "suo", "tai", "tan", "tao", "tei", "tie", "tou", "tui", "tun", "tuo", "wai", "wan", "wei",
        "wen", "xia", "xie", "xin", "xiu", "xue", "xun", "yai", "yan", "yao", "yin", "you", "yue",
        "yun", "zai", "zan", "zao", "zei", "zen", "zou", "zui", "zun", "zuo", "ang", "eng",
        // 2-letter
        "ba", "bo", "bi", "bu", "ca", "ce", "ci", "cu", "da", "de", "di", "du", "fa", "fo", "fu",
        "ga", "ge", "gu", "ha", "he", "hu", "ji", "ju", "ka", "ke", "ku", "la", "le", "li", "lo",
        "lu", "lv", "ma", "me", "mi", "mo", "mu", "na", "ne", "ni", "nu", "nv", "pa", "pi", "po",
        "pu", "qi", "qu", "re", "ri", "ru", "sa", "se", "si", "su", "ta", "te", "ti", "tu", "wa",
        "wo", "wu", "xi", "xu", "ya", "ye", "yi", "yu", "za", "ze", "zi", "zu", "ai", "an", "ao",
        "ei", "en", "er", "ou", // 1-letter
        "a", "e", "o",
    ]
}

/// Greedy longest-match split of plain ASCII pinyin into syllables.
fn split_pinyin_syllables(ascii: &str) -> Vec<&str> {
    let table = pinyin_syllable_table();
    let mut syllables = Vec::new();
    let mut pos = 0;
    let bytes = ascii.as_bytes();

    while pos < bytes.len() {
        let remaining = &ascii[pos..];
        let mut matched = false;
        for &syl in table {
            if remaining.starts_with(syl) {
                syllables.push(&ascii[pos..pos + syl.len()]);
                pos += syl.len();
                matched = true;
                break;
            }
        }
        if !matched {
            pos += 1;
        }
    }

    syllables
}

fn escape_rust_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn apply_compat_overrides(
    hanzi: &str,
    mut pinyin: String,
    mut meaning: String,
) -> (String, String) {
    match hanzi {
        "朋友" => {
            pinyin = "peng2you3".to_string();
            if meaning.is_empty() || meaning == "N" {
                meaning = "friend".to_string();
            }
        }
        "朋友们" => {
            pinyin = "peng2you3men5".to_string();
            if meaning.is_empty() || meaning == "N" {
                meaning = "friends".to_string();
            }
        }
        "对不起" => {
            pinyin = "dui4bu4qi3".to_string();
            if meaning.is_empty() {
                meaning = "sorry".to_string();
            }
        }
        "谢谢" => {
            if pinyin.is_empty() {
                pinyin = "xie4xie4".to_string();
            }
            if meaning.is_empty() {
                meaning = "thanks".to_string();
            }
        }
        _ => {}
    }
    (pinyin, meaning)
}

fn generate_decomposition_data() -> Result<(), String> {
    let radicals_str = "火 水 力 心 口 目 手 木 田 日 月 人 女 子 禾 十 金 土 又 寸 刀 言 足 糸 门 马 鸟 雨 石 虫 贝 山 犬 弓 食 衣 竹 走 车 王 大 小 工 白";
    let known_radicals: HashSet<char> = radicals_str
        .split_whitespace()
        .filter_map(|s| s.chars().next())
        .collect();

    let variant_map: HashMap<char, char> = [
        // === Standard radical variants (simplified forms) ===
        ('氵', '水'),
        ('扌', '手'),
        ('讠', '言'),
        ('钅', '金'),
        ('亻', '人'),
        ('忄', '心'),
        ('饣', '食'),
        ('衤', '衣'),
        ('刂', '刀'),
        ('犭', '犬'),
        ('灬', '火'),
        ('⺼', '月'),
        ('⺮', '竹'),
        ('纟', '糸'),
        ('艹', '木'), // grass radical → 木 (plant)
        ('辶', '走'),
        ('阝', '山'),
        ('礻', '衣'),
        ('⻊', '足'),
        ('⺈', '刀'),
        ('⺡', '水'),
        ('⺢', '水'),
        ('⺣', '火'),
        ('⺩', '王'),
        ('⻗', '雨'),
        ('⻝', '食'),
        ('⻟', '食'),
        ('⻖', '山'),
        ('⻏', '山'),
        ('⻞', '食'),
        ('冫', '水'),
        ('宀', '门'), // roof → 门 (shelter)
        ('彳', '足'),
        // REMOVED: ('攵', '手') — 攵 (rap/tap) ≠ ⺙; over-mapped
        ('夂', '足'),
        ('夊', '足'),
        // === Sub-component → known radical mappings ===
        ('甲', '田'),
        ('止', '足'),
        // REMOVED: ('夕', '月') — 夕 (evening) is distinct from 月 (moon); caused wrong decomp for 外, 多, 名
        // REMOVED: ('囗', '口') — 囗 (enclosure) ≠ 口 (mouth); caused wrong decomp for 回, 四, 国
        ('户', '门'),
        ('巾', '衣'),
        ('父', '人'),
        ('斤', '刀'),
        ('耂', '人'),
        ('卜', '十'),
        ('屮', '木'),
        // REMOVED: ('巳', '虫') — 巳 ≠ 己; caused wrong decomp for 起 (己→虫 is wrong)
        ('尸', '人'),
        ('勹', '力'),
        ('匕', '刀'),
        ('卩', '人'),
        // REMOVED: ('厶', '心') — 厶 (private) has no real link to 心; caused wrong decomp for 去, 动, 会, 能
        ('㐅', '十'),
        ('立', '人'),
        ('朩', '木'),
        // === Intermediate character → radical mappings (stop decomposition) ===
        // These prevent over-decomposition by treating multi-stroke components as units
        ('矢', '大'), // 矢 (arrow) → 大 (IDS: 𠂉+大); used in 知, 医, 候
        ('隹', '鸟'), // 隹 (short-tailed bird) → 鸟 (bird); used in 准, 谁, 难
        ('覀', '口'), // 覀 (cover/west top) → 口 (seen in 要, 票); HC treats as unit
        ('豕', '犬'), // 豕 (pig) → 犬 (animal); used in 家
        ('欠', '力'), // 欠 (yawn/lack) → 力 (effort); used in 次, 欢, 歌
        ('殳', '手'), // 殳 (weapon) → 手 (strike); used in 没
        ('⺙', '手'), // ⺙ (knock radical) → 手; used in 做, 放, 教 (distinct from 攵)
        ('戈', '刀'), // 戈 (halberd) → 刀 (blade); used in 我, 找
        ('广', '门'), // 广 (shelter) → 门 (building); used in 床, 店
        ('穴', '门'), // 穴 (cave) → 门 (opening); used in 穿
        ('疒', '人'), // 疒 (sickness) → 人 (affects person); used in 病
        ('龵', '手'), // 龵 (hand-top) → 手; used in 看
        ('示', '衣'), // 示 (altar/show) → 衣 (display); used in 票
        ('甘', '口'), // 甘 (sweet) → 口 (taste); used in 期
        ('曰', '日'), // 曰 (say) → 日 (similar shape); used in 最
        ('自', '目'), // 自 (self/nose) → 目 (face); used in 息
        ('罒', '目'), // 罒 (net-top) → 目 (eyes); used in 慢
        ('𧾷', '足'), // ⻊ variant → 足; used in 跑, 跟, 路
        ('士', '十'), // 士 (scholar) → 十; used in 喜
        ('匚', '工'), // 匚 (box) → 工 (container); used in 医
        ('弋', '弓'), // 弋 (shoot) → 弓 (projectile); used in 试
        ('冂', '门'), // 冂 (border) → 门 (enclosure); used in 再, 南
        ('龶', '十'), // 龶 → 十; used in 青
        ('疋', '足'), // 疋 (bolt of cloth / foot) → 足; used in 蛋
        ('爫', '手'), // 爫 (claw-top) → 手 (grasp); used in 爱, 菜
        ('⺺', '力'), // ⺺ → 力; used in 事
        ('⺌', '小'), // ⺌ (small-top) → 小; used in 常
        ('冖', '门'), // 冖 (cover) → 门 (shelter); used in 学, 觉
        ('见', '目'), // 见 (see) → 目 (eye); used in 现, 觉
        ('𠂇', '手'), // 𠂇 (left hand) → 手; used in 有, 左, 友
        ('⺍', '火'), // ⺍ (small-fire) → 火; used in 兴, 觉
    ]
    .into_iter()
    .collect();

    let ids_text = fs::read_to_string("data/ids.txt")
        .map_err(|e| format!("failed to read data/ids.txt: {e}"))?;

    let mut ids_map: HashMap<char, String> = HashMap::new();
    for line in ids_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 3 {
            let character = cols[1].chars().next().unwrap_or('\0');
            let first_decomposition = cols[2].to_string();
            if character != '\0' {
                ids_map.insert(character, first_decomposition);
            }
        }
    }

    let unique_chars_text = fs::read_to_string("data/unique_chars.txt")
        .map_err(|e| format!("failed to read data/unique_chars.txt: {e}"))?;

    let unique_chars: Vec<char> = unique_chars_text
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.chars().next())
        .collect();

    // Manual overrides: checked FIRST (before IDS decomposition).
    // For chars that are atomic (no IDS decomposition) or whose IDS decomposition
    // produces wrong results vs hanzicraft.com.
    let manual_overrides: HashMap<char, Vec<char>> = [
        // === Truly atomic chars (HC shows no sub-radicals) ===
        ('一', vec!['刀']),
        ('二', vec!['刀']),
        ('七', vec!['刀']),
        ('八', vec!['刀']),
        ('九', vec!['力']),
        ('也', vec!['水']),
        ('几', vec!['木']),
        ('出', vec!['山']),
        ('比', vec!['人']),
        ('毛', vec!['衣']),
        ('气', vec!['力']),
        ('生', vec!['土', '禾']),
        ('用', vec!['工']),
        ('回', vec!['口']),
        ('肉', vec!['食']),
        ('身', vec!['人']),
        ('飞', vec!['鸟']),
        // === Chars whose IDS decomposition is wrong per HC ===
        ('三', vec!['十']),       // HC: 一+二, but we want a combat skill
        ('上', vec!['土']),       // HC: ⺊+一 → 土 fits
        ('不', vec!['大']),       // HC: 一+丿+卜
        ('下', vec!['十']),       // HC: 一+卜 → 十 via 卜
        ('东', vec!['木', '小']), // HC: 乚+一+小, but means "east" → 木+小
        ('五', vec!['工']),       // HC: 一+力+一
        ('书', vec!['言']),       // HC: 丨+丶 → book = 言
        ('买', vec!['贝']),       // HC: ㇖+大+⺀ → buy = 贝
        ('了', vec!['刀']),       // HC: ㇇+亅
        ('儿', vec!['人']),       // HC: 儿
        ('元', vec!['人']),       // HC: 二+儿
        ('先', vec!['人', '足']), // HC: ⺧+儿
        ('六', vec!['大']),       // HC: 亠+八
        ('兴', vec!['火']),       // HC: ⺍+一+八
        ('再', vec!['又', '门']), // HC: 一+冂+土 → 冂→门
        ('写', vec!['言']),       // HC: 冖+一 → writing = 言
        ('开', vec!['门']),       // HC: 一+廾 → open = 门
        ('半', vec!['刀', '十']), // HC: 二+丨+丷
        ('干', vec!['十', '木']), // HC: 干 (atomic)
        ('年', vec!['禾']),       // HC: 丿+一+十+㇗+丨
        ('米', vec!['禾']),       // HC: 米 (atomic) → grain = 禾
        ('非', vec!['大']),       // HC: 非 (atomic)
        ('页', vec!['人']),       // HC: 页 (atomic)
        ('高', vec!['口']),       // HC: 高 (atomic) → tall structure has 口
        ('面', vec!['口']),       // HC: 面 (atomic) → face has 口
        ('风', vec!['力']),       // HC: 风 (atomic) → wind = 力
        ('网', vec!['糸']),       // HC: 网 (atomic) → net = 糸
        ('牛', vec!['力', '土']), // HC: 牛 (atomic)
        ('文', vec!['手']),       // HC: 文 (atomic) → writing = 手
        ('见', vec!['目']),       // HC: 见 (atomic) → see = 目
        ('旁', vec!['门', '土']), // HC: 亠+丷+冖+方
        ('方', vec!['土']),       // HC: 方 (atomic)
        ('觉', vec!['目', '火']), // HC: ⺍+冖+见 → ⺍=火, 见=目
        ('我', vec!['手', '刀']), // HC: 手+戈
        // === IDS gives wrong components; override with HC-correct radicals ===
        ('事', vec!['十', '口', '力']), // HC: 十+口+丨+⺺+亅 → 十, 口, ⺺→力
        ('果', vec!['田', '木']),       // HC: 田+木 (IDS wrongly gives 日+木)
        ('课', vec!['言', '田', '木']), // HC: 讠+田+木 (via 果→田+木)
        ('住', vec!['人', '王']),       // HC: 亻+王+丶 (IDS goes 主→亠+土, wrong)
        ('到', vec!['走', '刀']),       // HC: 至+刂 → 至 has 足/走 semantics
        ('条', vec!['足', '木']),       // HC: 夂+朩 → 夂=足, 朩=木 (IDS uses 朩[GT])
        ('重', vec!['十', '田']),       // HC: ㇒+十+里 → 十, 里→田
        ('起', vec!['走']),             // HC: 走+己 (己 is atomic, not a known radical)
        ('多', vec!['大']),             // HC: 多 (atomic) — was wrongly getting 月 via 夕→月
        ('外', vec!['大', '十']),       // HC: 夕+卜 → 卜→十; 夕 not a radical
        ('名', vec!['口']),             // HC: 夕+口 → 口 (夕 removed from variant_map)
        ('会', vec!['人']),             // HC: 人+二+厶 → just 人 (厶 no longer maps)
        ('去', vec!['土']),             // HC: 土+厶 → just 土 (厶 removed)
        ('动', vec!['力']),             // HC: 二+厶+力 → just 力 (厶 removed)
        ('包', vec!['力']),             // HC: 勹+巳 → 勹=力 (巳 removed)
        ('能', vec!['月']),             // HC: 厶+月 → 月 (厶removed)
        ('四', vec!['口']),             // HC: 囗+儿 → treat as 口
        ('国', vec!['口', '王']),       // HC: 囗+玉 → 口+王
        ('西', vec!['口']),             // HC: 西 = 覀 area → 口
        ('北', vec!['刀']),             // HC: 北 (atomic)
        ('里', vec!['田']),             // HC: 里 (has 甲→田 inside)
        ('正', vec!['足']),             // HC: 一+止 → 止=足
        ('老', vec!['人', '刀']),       // HC: 老 (has 耂=人, 匕=刀 inside)
        ('行', vec!['足']),             // HC: 行 (atomic) → travel = 足
        ('来', vec!['木']),             // HC: 木+一+丷 → just 木
        ('本', vec!['木']),             // HC: 木+一 → just 木
        // === Chars that IDS decomposes but misses components per HC ===
        ('体', vec!['人', '木']),       // HC: 亻+木+一 (IDS only found 人)
        ('妹', vec!['女', '木']),       // HC: 女+木+一
        ('姐', vec!['女', '月']),       // HC: 女+月+一
        ('真', vec!['十', '目']),       // HC: 十+目+一+八
        ('睡', vec!['目', '十']),       // HC: 目+㇒+十+士+艹
        ('晚', vec!['日', '刀', '口']), // HC: 日+⺈+口+丨+乚 → ⺈=刀
        ('跑', vec!['足', '力']),       // HC: ⻊+勹+巳 → 足+力
        ('跟', vec!['足']),             // HC: ⻊+艮
        ('路', vec!['足', '口']),       // HC: ⻊+夂+口
        ('新', vec!['人', '十', '刀']), // HC: 立+十+小+斤 → 立=人, 斤=刀, 十
        ('知', vec!['大', '口']),       // HC: 矢+口 → 矢=大
        ('穿', vec!['门']),             // HC: 穴+牙 → 穴=门
        ('师', vec!['衣']),             // HC: 一+巾 → 巾=衣
        ('帮', vec!['山', '衣']),       // HC: 一+二+丨+阝+巾 → 阝=山, 巾=衣
        ('常', vec!['口', '衣', '小']), // HC: ⺌+冖+口+巾 → ⺌=小, 口, 巾=衣
        ('票', vec!['口', '衣']),       // HC: 覀+示 → 覀=口, 示=衣
        ('站', vec!['人', '口']),       // HC: 立+⺊+口 → 立=人, 口
        ('钱', vec!['金', '刀']),       // HC: 钅+戋 → 金, 戋 has blade
        ('错', vec!['金', '木', '日']), // HC: 钅+艹+一+日 → 金, 艹=木, 日
        ('谢', vec!['言', '人', '寸']), // HC: 讠+身+寸 → 身=人
        ('哥', vec!['口']),             // HC: 哥 (atomic) → has 口
        ('朋', vec!['月']),             // HC: 朋 (atomic)
        ('歌', vec!['口', '力']),       // HC: 哥+欠 → 口+力
        ('医', vec!['工', '大']),       // HC: 匚+矢 → 匚=工, 矢=大
        ('那', vec!['山']),             // HC: ㇆+二+丨+阝 → 阝=山
        ('都', vec!['人', '日', '山']), // HC: 耂+日+阝 → 耂=人, 日, 阝=山
        ('候', vec!['人', '大']),       // HC: 亻+丨+矢 → 人, 矢=大
        ('假', vec!['人', '又']),       // HC: 亻+尸+二+又
        ('次', vec!['水', '力']),       // HC: 冫+欠 → 水+力
        ('岁', vec!['山']),             // HC: 山+夕 → 山 (夕 not mapped)
        ('图', vec!['口']),             // HC: 囗+夂+⺀
        ('备', vec!['足', '田']),       // HC: 夂+田 → 足+田
        ('床', vec!['门', '木']),       // HC: 广+木 → 门+木
        ('店', vec!['门', '口']),       // HC: 广+⺊+口 → 门+口
        ('房', vec!['门']),             // HC: 户+方 → 门
        ('病', vec!['人']),             // HC: 疒+一+人+冂
        ('看', vec!['手', '目']),       // HC: 龵+目 → 手+目
        ('考', vec!['人']),             // HC: 耂+一+㇉ → 人
        ('爱', vec!['手', '又']),       // HC: 爫+冖+𠂇+又 → 手+又
        ('菜', vec!['木', '手']),       // HC: 艹+爫+木 → 木+手
        ('话', vec!['言', '口']),       // HC: 讠+舌 → 言, 舌 has 口
        ('请', vec!['言', '月']),       // HC: 讠+青 → 言, 青→月
        ('谁', vec!['言', '鸟']),       // HC: 讠+隹 → 言, 隹=鸟
        ('难', vec!['又', '鸟']),       // HC: 又+隹 → 又, 隹=鸟
        ('准', vec!['水', '鸟']),       // HC: 冫+隹 → 水+鸟
        ('样', vec!['木']),             // HC: 木+羊 → just 木
        ('楼', vec!['木', '女']),       // HC: 木+米+女 → 木, 女 (米→禾 would add)
        ('期', vec!['口', '月']),       // HC: 甘+一+八+月 → 甘=口, 月
        ('星', vec!['日']),             // HC: 日+生 → just 日
        ('电', vec!['日']),             // HC: 日+丨+乚 → just 日
        ('饿', vec!['食', '手', '刀']), // HC: 饣+手+戈 → 食, 手, 戈=刀
        ('馆', vec!['食', '门']),       // HC: 饣+宀+㠯 → 食, 宀=门
        ('语', vec!['言', '口']),       // HC: 讠+一+力+一+口
        ('读', vec!['言', '十', '大']), // HC: 讠+十+㇖+大+⺀
        ('识', vec!['言', '口']),       // HC: 讠+口+八
        ('诉', vec!['言', '刀']),       // HC: 讠+斤+丶 → 斤=刀
        ('院', vec!['山', '门']),       // HC: 阝+宀+二+儿
        ('脑', vec!['月']),             // HC: 月+亠+凵+乂
        ('学', vec!['子', '门']),       // HC: ⺍+冖+子 → 子, 冖=门
        ('习', vec!['水']),             // HC: ㇆+亠
        ('蛋', vec!['足', '虫']),       // HC: 疋+虫 → 疋=足, 虫
        ('试', vec!['言', '弓', '工']), // HC: 讠+弋+工 → 弋=弓
        ('视', vec!['衣', '目']),       // HC: 礻+见 → 衣, 见=目
        ('么', vec!['口']),             // HC: 丿+厶 (both strokes) → question particle, 口 fits
        ('放', vec!['手']),             // HC: 方+⺙ → ⺙=手 (IDS uses 攵 which we don't map)
    ]
    .into_iter()
    .collect();

    let mut out = String::new();
    out.push_str("// @generated by build.rs; do not edit by hand.\n");

    // Helper function: decompose a single character
    out.push_str("fn components_for_char(ch: char) -> Vec<&'static str> {\n");
    out.push_str("    match ch {\n");

    for &ch in &unique_chars {
        // Manual overrides take precedence over IDS decomposition
        let deduplicated = if let Some(overrides) = manual_overrides.get(&ch) {
            overrides.clone()
        } else {
            let mut visited = HashSet::new();
            let raw_components =
                decompose(ch, &ids_map, &known_radicals, &variant_map, &mut visited);

            let mut deduped = Vec::new();
            for comp in raw_components {
                if !deduped.contains(&comp) {
                    deduped.push(comp);
                }
                if deduped.len() >= 4 {
                    break;
                }
            }
            deduped
        };

        if !deduplicated.is_empty() {
            out.push_str(&format!("        '{}' => vec![", ch));
            for (i, comp) in deduplicated.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("\"{}\"", comp));
            }
            out.push_str("],\n");
        }
    }

    out.push_str("        _ => vec![],\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Main function: iterate all characters and collect components
    out.push_str("pub fn get_components(hanzi: &str) -> Vec<&'static str> {\n");
    out.push_str("    let mut result = Vec::new();\n");
    out.push_str("    for ch in hanzi.chars() {\n");
    out.push_str("        for comp in components_for_char(ch) {\n");
    out.push_str("            if !result.contains(&comp) {\n");
    out.push_str("                result.push(comp);\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("    result\n");
    out.push_str("}\n");

    let out_dir = env::var("OUT_DIR").map_err(|e| format!("OUT_DIR not set: {e}"))?;
    let out_path = Path::new(&out_dir).join("decomposition_data.rs");
    fs::write(out_path, out)
        .map_err(|e| format!("failed to write generated decomposition_data.rs: {e}"))?;

    Ok(())
}

fn decompose(
    ch: char,
    ids_map: &HashMap<char, String>,
    known_radicals: &HashSet<char>,
    variant_map: &HashMap<char, char>,
    visited: &mut HashSet<char>,
) -> Vec<char> {
    let normalized = variant_map.get(&ch).copied().unwrap_or(ch);

    if known_radicals.contains(&normalized) {
        return vec![normalized];
    }

    if visited.contains(&ch) {
        return vec![];
    }
    visited.insert(ch);

    if let Some(ids) = ids_map.get(&ch) {
        let mut result = Vec::new();
        for component_char in ids.chars() {
            if ('\u{2FF0}'..='\u{2FFB}').contains(&component_char) {
                continue;
            }
            let sub = decompose(
                component_char,
                ids_map,
                known_radicals,
                variant_map,
                visited,
            );
            for r in sub {
                if !result.contains(&r) {
                    result.push(r);
                }
            }
        }
        visited.remove(&ch);
        return result;
    }

    visited.remove(&ch);
    vec![]
}
