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
//! What a person knows is still `Agent::found_out`, and how a thing is made is
//! still `making.rs`; the catalogue does not decide either. It says where in
//! the document each of those things sits, which is what the tech-tree panel
//! shows and what `technology_catalogue_tests` holds the making chain to.

use std::collections::BTreeMap;
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
