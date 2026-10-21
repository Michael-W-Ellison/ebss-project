// src/analytics/tests/technology_catalogue_tests.rs
//! The technology catalogue, held to the making chain.
//!
//! `data/technology.toml` is tech.txt as data: every listing of every stage
//! of every path, each with an id, and on the ones the model already carries
//! a `model` link saying what it is here - a recipe, a garment, a building, an
//! action, or something everybody is born knowing. Nothing in the simulation
//! reads it yet. What it is for, before anything changes, is to say exactly
//! where the document and the model agree and where they do not, so that a
//! change made later is made against a list rather than against a memory.
//!
//! Two kinds of thing are here. The checks fail the suite: a link to a recipe
//! that does not exist, a product or garment no listing claims, a stage-0
//! listing that is a discovery here without saying why, a gate in
//! `what_has_to_be_known_first` the catalogue does not carry. The report -
//! `what_the_catalogue_says_about_the_model` - only prints: how much of each
//! path is carried, and every place the model gives a thing away earlier, or
//! holds it back later, than the document puts it.

use std::collections::{BTreeMap, BTreeSet};

use crate::agents::equipment::GARMENT_RECIPES;
use crate::agents::Agent;
use crate::environment::catalogue::{self, the_catalogue, Catalogue, Tech};
use crate::environment::making::{self, EVERY_STEP, EVERY_WORKING};
use crate::environment::stage::EVERY_PATH;
use crate::environment::Action;
use crate::world::BuildingType;

fn catalogue() -> &'static Catalogue {
    the_catalogue()
}

/// The drives a listing may name. Only the ones the document needs so far.
const DRIVES: &[&str] = &["luxury"];

/// Every product the making chain can put in a hand, recipes and workings both.
fn every_product() -> BTreeSet<&'static str> {
    EVERY_STEP
        .iter()
        .map(|m| m.makes)
        .chain(EVERY_WORKING.iter().map(|w| w.makes))
        .collect()
}

/// Products everybody can make without finding anything out first.
fn obvious_products() -> BTreeSet<&'static str> {
    EVERY_STEP
        .iter()
        .filter(|m| m.born_known())
        .map(|m| m.makes)
        .chain(EVERY_WORKING.iter().filter(|w| w.born_known()).map(|w| w.makes))
        .collect()
}

fn every_garment() -> BTreeSet<&'static str> {
    GARMENT_RECIPES.iter().map(|g| g.id).collect()
}

/// The name a value goes by in its `Debug`, without its fields.
fn variant_name(debug: String) -> String {
    debug
        .split(|c| c == ' ' || c == '{' || c == '(')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// The buildings a listing may name. Spelled as values so a renamed variant
/// breaks the build here rather than leaving a stale name in the catalogue.
fn every_building() -> BTreeSet<String> {
    use BuildingType::*;
    [
        Windbreak, LeanTo, BrushHut, SkinTent, Burrow, Longhouse, UpgradedLonghouse, SmallHouse,
        MediumHouse, LargeHouse,
        Manor, TownCenter, TownStorage, GuardPost, Workshop, Forge, Smithy, Bakery, WeaverHut,
        PotteryKiln, Tannery, Mill, Butchery, Brewery, Dairy, Glassworks, Dyeworks, Ropewalk,
        Brickyard, PaperMill, TailorShop, CobblerShop, BarberShop, Scriptorium, Storehouse, Farm,
        AnimalPen, Shrine, Temple, MedicalBuilding,
    ]
    .into_iter()
    .map(|b| variant_name(format!("{b:?}")))
    .collect()
}

/// The actions a listing may name, spelled as values for the same reason.
fn every_action() -> BTreeSet<String> {
    let nobody = uuid::Uuid::nil();
    let s = String::new;
    [
        Action::Eat { food_type: s() },
        Action::Sleep { duration: 0 },
        Action::Gather { resource_type: s() },
        Action::HarvestPlant { plant_id: nobody },
        Action::Hunt { animal_id: nobody, weapon: None },
        Action::Tame { animal_id: nobody, food_type: None },
        Action::CollectAnimalProduct { animal_id: nobody },
        Action::SeekShelter,
        Action::LightFire,
        Action::Cook { food_type: s() },
        Action::MakeClothing { garment: s() },
        Action::WearClothing { garment: s() },
        Action::ScatterSeed,
        Action::TillSoil,
        Action::TendField,
        Action::Dry { what: s() },
        Action::Smoke { what: s() },
        Action::Brine { what: s() },
        Action::Cache,
        Action::PutUpRack,
        Action::HangUp { what: s() },
        Action::StoneBoil,
        Action::PitOven,
        Action::LayBedding,
        Action::Boil,
        Action::Salt { what: s() },
        Action::Excavate,
        Action::Cover { what: s() },
        Action::Trade { with: nobody },
        Action::GiveTo { to: nobody },
        Action::TakeCutting,
        Action::PlantCutting,
        Action::SpreadMuck,
        Action::Fish,
        Action::ForageShallows,
        Action::SetFishTrap,
        Action::CheckFishTraps,
        Action::SetSnare,
        Action::CheckSnares,
        Action::Treat { who: None },
    ]
    .into_iter()
    .map(|a| variant_name(format!("{a:?}")))
    .collect()
}

/// Whether a `model` link names something that exists.
fn names_something_real(link: &str) -> Result<(), String> {
    let (kind, what) = link
        .split_once(':')
        .ok_or_else(|| format!("{link}: a model link is kind:name"))?;
    let real = match kind {
        "make" => every_product().contains(what),
        "wear" => every_garment().contains(what),
        "build" => every_building().contains(what),
        "do" => every_action().contains(what),
        "born" => Agent::what_anybody_is_born_knowing().contains(what),
        _ => return Err(format!("{link}: no such kind as {kind}")),
    };
    if real {
        Ok(())
    } else {
        Err(format!("{link}: nothing in the model is called {what}"))
    }
}

// --------------------------------------------------------------------------
// The catalogue is the document
// --------------------------------------------------------------------------

/// Thirty-five paths, in the stage table's order, every stage with something
/// in it, and every listing filed under the stage its id says.
#[test]
fn the_catalogue_is_the_whole_document() {
    let cat = catalogue();
    assert_eq!(cat.path.len(), EVERY_PATH.len(), "one row per development path");
    for (row, path) in cat.path.iter().zip(EVERY_PATH) {
        assert_eq!(row.model, format!("{path:?}"), "{} is in the stage table's place", row.id);
    }

    let stages: BTreeMap<&str, usize> =
        cat.path.iter().map(|p| (p.id.as_str(), p.stages.len())).collect();
    let mut seen = BTreeSet::new();
    let mut filled = BTreeSet::new();
    for t in &cat.tech {
        assert!(seen.insert(t.id.as_str()), "{} is listed twice", t.id);
        let n = *stages
            .get(t.path.as_str())
            .unwrap_or_else(|| panic!("{}: no path called {}", t.id, t.path));
        assert!(t.stage < n, "{}: {} has no stage {}", t.id, t.path, t.stage);
        assert!(
            t.id.starts_with(&format!("{}.{}.", t.path, t.stage)),
            "{}: the id is path.stage.name",
            t.id
        );
        assert!(
            ["technique", "product", "food", "example"].contains(&t.role.as_str()),
            "{}: no such role as {}",
            t.id,
            t.role
        );
        assert!(!t.name.is_empty(), "{} has a name", t.id);
        filled.insert((t.path.as_str(), t.stage));
    }
    for p in &cat.path {
        for s in 0..p.stages.len() {
            assert!(filled.contains(&(p.id.as_str(), s)), "{} stage {s} lists nothing", p.id);
        }
    }
    assert_eq!(cat.tech.len(), 1461, "every listing in tech.txt");
}

// --------------------------------------------------------------------------
// The links are real
// --------------------------------------------------------------------------

/// Every `model` link names a recipe, garment, building, action or birthright
/// that exists, and every `same_as` and `needs` names a listing that does.
#[test]
fn every_link_names_something_that_exists() {
    let cat = catalogue();
    let ids: BTreeSet<&str> = cat.tech.iter().map(|t| t.id.as_str()).collect();
    let mut wrong = Vec::new();
    for t in &cat.tech {
        for link in &t.model {
            if let Err(e) = names_something_real(link) {
                wrong.push(format!("{}: {e}", t.id));
            }
        }
        for other in t.same_as.iter().chain(&t.needs) {
            if !ids.contains(other.as_str()) {
                wrong.push(format!("{}: no listing called {other}", t.id));
            }
            if other == &t.id {
                wrong.push(format!("{}: points at itself", t.id));
            }
        }
        if let Some(d) = &t.drive {
            if !DRIVES.contains(&d.as_str()) {
                wrong.push(format!("{}: no drive called {d}", t.id));
            }
        }
    }
    for u in &cat.unlisted {
        if let Err(e) = names_something_real(&u.model) {
            wrong.push(format!("unlisted {e}"));
        }
    }
    assert!(wrong.is_empty(), "links that go nowhere:\n  {}", wrong.join("\n  "));
}

/// Everything the making chain can make, and every garment, is claimed by a
/// listing, or set aside in `[[unlisted]]` with a reason - and nothing is
/// both, because a reason that no longer holds is worse than none.
#[test]
fn every_product_and_garment_has_a_place_in_the_document() {
    let cat = catalogue();
    let claimed: BTreeSet<&str> =
        cat.tech.iter().flat_map(|t| t.model.iter().map(String::as_str)).collect();
    let unlisted: BTreeSet<&str> = cat.unlisted.iter().map(|u| u.model.as_str()).collect();

    let wanted: Vec<String> = every_product()
        .into_iter()
        .map(|p| format!("make:{p}"))
        .chain(every_garment().into_iter().map(|g| format!("wear:{g}")))
        .collect();
    let homeless: Vec<&String> = wanted
        .iter()
        .filter(|w| !claimed.contains(w.as_str()) && !unlisted.contains(w.as_str()))
        .collect();
    assert!(homeless.is_empty(), "no listing claims {homeless:?}; link one or add it to [[unlisted]]");

    let both: Vec<&&str> = unlisted.iter().filter(|u| claimed.contains(**u)).collect();
    assert!(both.is_empty(), "{both:?} are claimed and also set aside as unlisted");
    for u in &cat.unlisted {
        assert!(!u.reason.trim().is_empty(), "unlisted {} says why", u.model);
    }
}

// --------------------------------------------------------------------------
// The document and the model agree, or say why not
// --------------------------------------------------------------------------

/// A stage-0 listing is born knowledge. Where the model makes that thing only
/// by finding it out, the listing says so - `innate = false` - and why, so
/// the disagreement is a decision on record rather than an accident.
#[test]
fn a_stage_zero_listing_that_is_a_discovery_here_says_why() {
    let cat = catalogue();
    let obvious = obvious_products();
    let mut unexplained = Vec::new();
    for t in &cat.tech {
        if t.innate.is_some() {
            assert_eq!(t.stage, 0, "{}: only a stage-0 listing can stop being innate", t.id);
            assert!(t.note.is_some(), "{}: innate = false wants a note saying why", t.id);
        }
        let discovered: Vec<&str> = t.makes().filter(|p| !obvious.contains(p)).collect();
        if t.innate() && !discovered.is_empty() {
            unexplained.push(format!("{} -> {discovered:?}", t.id));
        }
    }
    assert!(
        unexplained.is_empty(),
        "born knowledge in the document that the model makes a discovery:\n  {}",
        unexplained.join("\n  ")
    );
}

/// A product of the making chain is born knowledge exactly when the catalogue
/// says so, and a way of making it that is not the plain way is never born
/// knowledge: it is a variant to be worked out (flint for stone).
#[test]
fn what_is_born_knowledge_is_the_catalogues_to_say() {
    for product in every_product() {
        let catalogue_says = catalogue::born_knowing_how_to_make(product);
        let model_says = obvious_products().contains(product);
        assert_eq!(
            model_says, catalogue_says,
            "{product}: the making chain and the catalogue disagree about whether it is born knowledge"
        );
    }
    for step in EVERY_STEP.iter().filter(|s| !s.obvious) {
        assert!(
            EVERY_STEP.iter().any(|other| other.makes == step.makes && other.obvious),
            "{} has a variant way and no plain one",
            step.makes
        );
    }
}

/// Born knowledge can be made from born knowledge: a thing everybody knows
/// how to make has a born way whose made inputs everybody knows how to make
/// too. A knapped edge went to Stage 0 for this - the hand axe wants one.
#[test]
fn born_knowledge_can_be_made_from_born_knowledge() {
    let products = every_product();
    let born = obvious_products();
    let mut short = Vec::new();
    for product in &born {
        let by_making = EVERY_STEP.iter().filter(|s| s.makes == *product && s.born_known()).any(|s| {
            s.needs.iter().all(|(what, _)| !products.contains(what) || born.contains(what))
        });
        let by_working = EVERY_WORKING
            .iter()
            .filter(|w| w.makes == *product && w.born_known())
            .any(|w| !products.contains(w.to) || born.contains(w.to));
        if !by_making && !by_working {
            short.push(*product);
        }
    }
    assert!(short.is_empty(), "born knowledge that wants something found out to make: {short:?}");
}

/// A `needs` that gates something points at a listing that is one thing in
/// the model - a product, or an action - so "what has to be known first" is
/// a list of things, not of choices.
#[test]
fn a_gate_names_one_thing() {
    let cat = catalogue();
    let by_id: BTreeMap<&str, &Tech> = cat.tech.iter().map(|t| (t.id.as_str(), t)).collect();
    for t in cat.tech.iter().filter(|t| !t.model.is_empty()) {
        for n in &t.needs {
            let target = by_id[n.as_str()];
            assert_eq!(
                target.model.len(),
                1,
                "{} needs {n}, which is {:?} in the model: a gate is one thing",
                t.id,
                target.model
            );
        }
    }
}

/// Nothing waits, however far round, on itself.
#[test]
fn no_gate_waits_on_itself() {
    fn reaches(from: &str, to: &str, depth: usize) -> bool {
        depth < 32
            && making::what_has_to_be_known_first(from)
                .iter()
                .any(|first| *first == to || reaches(first, to, depth + 1))
    }
    for product in every_product() {
        assert!(!reaches(product, product, 0), "{product} waits on itself");
    }
}

/// Born knowledge waits on nothing; a garment that is not born knowledge
/// waits on something, or it would be born knowledge by another name.
#[test]
fn what_is_found_out_waits_on_something_and_what_is_born_on_nothing() {
    for product in obvious_products() {
        assert!(
            making::what_has_to_be_known_first(product).is_empty(),
            "{product} is born knowledge and waits on {:?}",
            making::what_has_to_be_known_first(product)
        );
    }
    for garment in every_garment() {
        let link = format!("wear:{garment}");
        if !catalogue::is_born_knowledge(&link) {
            assert!(
                !catalogue::what_comes_before(&link).is_empty(),
                "{garment} is not born knowledge and waits on nothing, so nobody would ever make it"
            );
        }
    }
}

/// The gates decided and measured before the catalogue held them (#66, #336)
/// are the ones it holds.
#[test]
fn the_wheel_and_the_metal_wait_where_they_were_put() {
    let first = making::what_has_to_be_known_first;
    assert_eq!(first("wheel"), &["metalaxe", "stoneware"]);
    assert_eq!(first("handcart"), &["wheel"]);
    assert_eq!(first("shinylump"), &["stoneware"]);
}

/// Stages 0 to 2 of the survival paths, as the model carries them. Stage 0 is
/// what a people arrives knowing; later stages are found out, and wait where
/// tech.txt's common prerequisites put them.
#[test]
fn the_survival_paths_start_where_the_document_starts_them() {
    let born = obvious_products();
    for product in [
        "lashing", "flint", "knappedtip", "handaxe", "diggingstick", "sharpenedstick",
        "leather", "meatstrips", "fishstrips", "bowl",
    ] {
        assert!(born.contains(product), "{product} is Stage 0");
    }
    for product in [
        "basket", "leatherbag", "fishingrod", "sling", "stoneknife", "spear", "shovel",
        "travois", "bow", "fishingnet", "claypot", "tinder", "flour",
    ] {
        assert!(!born.contains(product), "{product} is found out");
    }
    let first = making::what_has_to_be_known_first;
    assert_eq!(first("spear"), &["stoneknife"], "hafting a point wants a knife");
    assert_eq!(first("shovel"), &["stoneknife"]);
    assert_eq!(first("bow"), &["stoneknife"]);
    assert_eq!(first("fishingnet"), &["fishingrod"], "lines before nets");
    assert_eq!(first("ale"), &["bowl"], "a container before fermenting");
    assert_eq!(first("bricks"), &["stoneware"]);

    let garment = |g: &str| catalogue::what_comes_before(&format!("wear:{g}")).to_vec();
    assert!(catalogue::is_born_knowledge("wear:fur_coat"));
    assert!(catalogue::is_born_knowledge("wear:bark_cloak"));
    assert_eq!(garment("leather_tunic"), vec!["make:stoneknife"], "sewn: an awl, which is the knife");
    assert_eq!(garment("leather_pants"), vec!["make:stoneknife"]);
    assert_eq!(garment("wool_cloak"), vec!["make:stoneknife"], "cut to shape: a knife");
    assert_eq!(garment("hide_armor"), vec!["make:stoneknife"]);
    assert_eq!(garment("linen_cloak"), vec!["make:rettedflax"], "linen is retted flax");
    assert_eq!(garment("linen_shirt"), vec!["make:rettedflax", "make:stoneknife"]);
}

/// The status goods the design names - figurines, ceremonial dress and
/// embroidery - answer the luxury drive, wherever the document lists them.
#[test]
fn the_status_goods_answer_the_luxury_drive() {
    let cat = catalogue();
    let mut found = 0;
    for t in &cat.tech {
        let name = t.name.to_lowercase();
        if name.contains("figurine") || name.contains("ceremonial dress") || name.contains("embroidery") {
            found += 1;
            assert_eq!(t.drive.as_deref(), Some("luxury"), "{} is a status good", t.id);
        }
    }
    assert!(found >= 4, "the document lists them ({found} found)");
}

/// The agricultural path in three stages (#346). Gathering is born; scattering
/// gathered seed is Stage 1 and found out; a garden is Stage 2, found out, and
/// waits on sowing; cuttings and muck come with gardens.
#[test]
fn agriculture_comes_in_the_documents_three_stages() {
    use crate::agents::{AgentConfig, Population};
    assert!(catalogue::is_born_knowledge("do:HarvestPlant"));
    assert!(catalogue::is_born_knowledge("do:Gather"));
    for later in ["do:ScatterSeed", "do:TillSoil", "do:TendField", "do:PlantCutting", "do:SpreadMuck"] {
        assert!(!catalogue::is_born_knowledge(later), "{later} is found out");
    }
    assert_eq!(catalogue::what_comes_before("do:TillSoil"), &["do:ScatterSeed"]);
    assert_eq!(catalogue::what_comes_before("do:TendField"), &["do:ScatterSeed"]);
    assert_eq!(catalogue::what_comes_before("do:PlantCutting"), &["do:TillSoil"]);
    assert_eq!(catalogue::what_comes_before("do:SpreadMuck"), &["do:TillSoil"]);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &mut population.agents[0];
    for action in ["ScatterSeed", "TillSoil", "TendField", "PlantCutting", "SpreadMuck"] {
        assert!(!founder.knows_how_to_do(action), "a founder is a gatherer: {action}");
    }

    founder.found_out_how_to("do:ScatterSeed");
    assert!(founder.knows_how_to_do("TendField"), "weeding comes with sowing");
    assert!(
        !founder.knows_how_to_do("TillSoil"),
        "a garden is found out by itself, not handed over with sowing"
    );
    assert!(!founder.knows_how_to_do("PlantCutting"));

    founder.found_out_how_to("do:TillSoil");
    assert!(founder.knows_how_to_do("PlantCutting"), "cuttings come with gardens");
    assert!(founder.knows_how_to_do("SpreadMuck"), "and so does muck");
}

// --------------------------------------------------------------------------
// The report
// --------------------------------------------------------------------------

/// How much of each path the model carries, and where it disagrees with the
/// document about when a thing arrives. Prints; checks only that it ran.
///
///     cargo test --release --lib what_the_catalogue_says -- --nocapture
#[test]
fn what_the_catalogue_says_about_the_model() {
    let cat = catalogue();
    let obvious = obvious_products();

    println!("\n== Carried: listings with a model link, stages 0 / 1 / 2 / later ==");
    for p in &cat.path {
        let mut linked = [0usize; 4];
        let mut total = [0usize; 4];
        for t in cat.tech.iter().filter(|t| t.path == p.id) {
            let band = t.stage.min(3);
            total[band] += 1;
            if !t.model.is_empty() {
                linked[band] += 1;
            }
        }
        let cells: Vec<String> = (0..4).map(|b| format!("{:>2}/{:<2}", linked[b], total[b])).collect();
        println!("  {:<45} {}", p.id, cells.join("  "));
    }

    // The earliest stage the document puts each model thing at.
    let earliest: BTreeMap<&str, (usize, &str)> = cat
        .earliest_of_each()
        .into_iter()
        .map(|(link, t)| (link, (t.stage, t.id.as_str())))
        .collect();

    println!("\n== Born knowing here, later in the document (the model gives it away early) ==");
    for (link, (stage, id)) in &earliest {
        let given = match link.split_once(':') {
            Some(("make", p)) => obvious.contains(p),
            Some(("wear", _)) => true,
            _ => false,
        };
        if given && *stage > 0 {
            println!("  {link:<22} stage {stage}  {id}");
        }
    }

    println!("\n== A discovery here, born knowing in the document (the model holds it back) ==");
    for t in cat.tech.iter().filter(|t| t.stage == 0) {
        for p in t.makes().filter(|p| !obvious.contains(p)) {
            println!("  make:{p:<17} {}  ({})", t.id, t.note.as_deref().unwrap_or(""));
        }
    }

    println!("\n== Discoveries here, and the stage the document puts them at ==");
    for (link, (stage, id)) in &earliest {
        if let Some(("make", p)) = link.split_once(':') {
            if !obvious.contains(p) {
                println!("  {link:<22} stage {stage}  {id}");
            }
        }
    }

    println!("\n== Status goods (luxury drive) ==");
    for t in cat.tech.iter().filter(|t| t.drive.as_deref() == Some("luxury")) {
        println!("  {}", t.id);
    }

    println!("\n== Set aside: the model's own things the document has no line for ==");
    for u in &cat.unlisted {
        println!("  {:<22} {}", u.model, u.reason);
    }
    assert!(!earliest.is_empty());
}

/// The fishing path's first two stages: picking over the shallows is born
/// knowledge, and a trap is found out, with setting and emptying one coming
/// with it.
#[test]
fn the_fishing_path_starts_in_the_shallows() {
    assert!(catalogue::is_born_knowledge("do:ForageShallows"));
    assert!(catalogue::is_born_knowledge("do:Fish"), "spearing in shallow water is Stage 0");
    assert!(!catalogue::born_knowing_how_to_make("fishtrap"), "a trap is Stage 1");
    for doing in ["do:SetFishTrap", "do:CheckFishTraps"] {
        assert!(!catalogue::is_born_knowledge(doing));
        assert_eq!(catalogue::what_comes_before(doing), ["make:fishtrap"], "{doing}");
    }
}
