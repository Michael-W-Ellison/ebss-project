// src/environment/catalogue.rs
//! The technology catalogue: tech.txt as data.
//!
//! `data/technology.toml` holds every listing of every stage of the
//! thirty-five development paths in tech.txt, each with an id
//! (`path.stage.name`), and on the ones this model carries a `model` link
//! saying what the listing is here: `make:<product>` for a step in the making
//! chain, `wear:<garment>`, `build:<BuildingType>`, `do:<action>` or
//! `born:<what everybody is born knowing>`. It is written by
//! `tools/tech_txt_to_catalogue.py`, whose docstring says what each field
//! means.
//!
//! **It decides when a thing can be known.** How a thing is made is
//! `making.rs`, and what a person has found out is `Agent::found_out`; what
//! everybody is born knowing, and what has to be known before something else
//! can be, is here:
//!
//! - A product or garment is born knowledge when one of its listings is
//!   innate - Stage 0, unless the listing says otherwise and why - or when
//!   the document has no line for it at all (`[[unlisted]]`).
//! - Everything else is found out, and waits on the products of the listings
//!   its own listings `need`. The document's own common prerequisites are
//!   where most of those come from: cordage and a needle before sewing, a
//!   container before fermenting, lines before nets.
//!
//! `technology_catalogue_tests` holds the making chain to it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::Deserialize;

const CATALOGUE: &str = include_str!("../../data/technology.toml");

#[derive(Debug, Deserialize)]
pub struct Catalogue {
    pub path: Vec<PathRow>,
    pub tech: Vec<Tech>,
    #[serde(default)]
    pub unlisted: Vec<Unlisted>,
}

/// One development path, and what each of its stages is called.
#[derive(Debug, Deserialize)]
pub struct PathRow {
    pub id: String,
    pub name: String,
    /// The `stage::Path` it is, by name.
    pub model: String,
    pub stages: Vec<String>,
}

/// One listing under one stage of one path.
#[derive(Debug, Deserialize)]
pub struct Tech {
    pub id: String,
    pub name: String,
    pub path: String,
    pub stage: usize,
    /// technique, product, food or example, as the document files it
    pub role: String,
    pub group: Option<String>,
    #[serde(default)]
    pub model: Vec<String>,
    #[serde(default)]
    pub same_as: Vec<String>,
    #[serde(default)]
    pub needs: Vec<String>,
    pub drive: Option<String>,
    pub innate: Option<bool>,
    pub note: Option<String>,
}

impl Tech {
    /// Stage 0 is what everybody is born knowing, unless the listing says
    /// otherwise and why.
    pub fn innate(&self) -> bool {
        self.innate.unwrap_or(self.stage == 0)
    }

    /// The products of the making chain this listing is.
    pub fn makes(&self) -> impl Iterator<Item = &str> {
        self.model.iter().filter_map(|m| m.strip_prefix("make:"))
    }
}

/// Something of the model's that the document has no line for, and why.
#[derive(Debug, Deserialize)]
pub struct Unlisted {
    pub model: String,
    pub reason: String,
}

impl Catalogue {
    /// The earliest listing for a model link (`make:wheel`), and its stage.
    pub fn earliest(&self, link: &str) -> Option<&Tech> {
        self.tech
            .iter()
            .filter(|t| t.model.iter().any(|m| m == link))
            .min_by_key(|t| t.stage)
    }

    /// The earliest listing for every model link the catalogue carries.
    pub fn earliest_of_each(&self) -> BTreeMap<&str, &Tech> {
        let mut out: BTreeMap<&str, &Tech> = BTreeMap::new();
        for t in &self.tech {
            for link in &t.model {
                let e = out.entry(link.as_str()).or_insert(t);
                if t.stage < e.stage {
                    *e = t;
                }
            }
        }
        out
    }

    pub fn path(&self, id: &str) -> Option<&PathRow> {
        self.path.iter().find(|p| p.id == id)
    }
}

/// The catalogue, read once.
pub fn the_catalogue() -> &'static Catalogue {
    static CAT: OnceLock<Catalogue> = OnceLock::new();
    CAT.get_or_init(|| toml::from_str(CATALOGUE).expect("data/technology.toml reads"))
}

/// What the catalogue says about knowing things, worked out once.
struct Knowing {
    /// Model links everybody is born knowing (`make:lashing`, `wear:fur_coat`)
    born: BTreeSet<&'static str>,
    /// For each model link, the model links that have to be known first
    /// (`make:stoneknife`, `do:ScatterSeed`)
    gates: BTreeMap<&'static str, Vec<&'static str>>,
}

fn knowing() -> &'static Knowing {
    static KNOWING: OnceLock<Knowing> = OnceLock::new();
    KNOWING.get_or_init(|| {
        let cat = the_catalogue();
        let by_id: BTreeMap<&str, &Tech> = cat.tech.iter().map(|t| (t.id.as_str(), t)).collect();

        let mut born: BTreeSet<&'static str> = cat.unlisted.iter().map(|u| u.model.as_str()).collect();
        let mut gates: BTreeMap<&'static str, BTreeSet<&'static str>> = BTreeMap::new();
        for t in &cat.tech {
            for link in &t.model {
                if t.innate() {
                    born.insert(link.as_str());
                }
                for needed in t.needs.iter().filter_map(|n| by_id.get(n.as_str())) {
                    for first in needed.model.iter().filter(|first| *first != link) {
                        gates.entry(link.as_str()).or_default().insert(first.as_str());
                    }
                }
            }
        }
        Knowing {
            born,
            gates: gates.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect(),
        }
    })
}

/// Whether everybody is born knowing a model link (`make:basket`).
pub fn is_born_knowledge(link: &str) -> bool {
    knowing().born.contains(link)
}

/// The model links that have to be known before a model link can be.
pub fn what_comes_before(link: &str) -> &'static [&'static str] {
    knowing().gates.get(link).map(Vec::as_slice).unwrap_or(&[])
}

/// Whether everybody is born knowing how to make a product of the making chain.
pub fn born_knowing_how_to_make(product: &str) -> bool {
    static BORN: OnceLock<BTreeSet<&'static str>> = OnceLock::new();
    BORN.get_or_init(|| knowing().born.iter().filter_map(|l| l.strip_prefix("make:")).collect())
        .contains(product)
}

/// The products that have to be known before a product can be found out.
pub fn what_comes_before_making(product: &str) -> &'static [&'static str] {
    static GATES: OnceLock<BTreeMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    GATES
        .get_or_init(|| {
            knowing()
                .gates
                .iter()
                .filter_map(|(l, v)| {
                    let product = l.strip_prefix("make:")?;
                    Some((product, v.iter().filter_map(|g| g.strip_prefix("make:")).collect()))
                })
                .collect()
        })
        .get(product)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
