use std::{
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex, OnceLock},
};

use fst::IntoStreamer;

// Hold the global key count in a mutex.
lazy_static! {
    static ref KEY_COUNT: Mutex<usize> = Mutex::new(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FstKey(usize);

#[derive(Debug, Clone)]
pub struct KeyedFst {
    fst: Arc<fst::Set<Vec<u8>>>,
    key: FstKey,
}

impl Hash for KeyedFst {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialEq for KeyedFst {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for KeyedFst {}

impl KeyedFst {
    pub fn new(fst: fst::Set<Vec<u8>>) -> Self {
        let mut key_count = KEY_COUNT.lock().unwrap();
        let key = FstKey(*key_count);
        *key_count += 1;
        Self {
            fst: Arc::new(fst),
            key,
        }
    }

    pub fn key(&self) -> FstKey {
        self.key
    }

    pub fn fst(&self) -> &fst::Set<Vec<u8>> {
        &self.fst
    }
}

static NEARBY_WORDS_FST: OnceLock<KeyedFst> = OnceLock::new();
static CATEGORY_WORDS_FST: OnceLock<KeyedFst> = OnceLock::new();
static STREET_SUFFIXES_FST: OnceLock<KeyedFst> = OnceLock::new();
static LOCALITIES_FST: OnceLock<KeyedFst> = OnceLock::new();
static SUBLOCALITY_FST: OnceLock<KeyedFst> = OnceLock::new();
static REGIONS_FST: OnceLock<KeyedFst> = OnceLock::new();
static COUNTRIES_FST: OnceLock<KeyedFst> = OnceLock::new();
static INTERSECTION_JOIN_WORDS_FST: OnceLock<KeyedFst> = OnceLock::new();
static BRICK_AND_MORTAR_WORDS: OnceLock<HashSet<String>> = OnceLock::new();

pub fn nearby_words_fst() -> KeyedFst {
    NEARBY_WORDS_FST
        .get_or_init(|| {
            KeyedFst::new(fst::Set::new(include_bytes!("../dicts/en/near.fst").to_vec()).unwrap())
        })
        .clone()
}

pub fn category_words_fst() -> KeyedFst {
    CATEGORY_WORDS_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/category.fst").to_vec()).unwrap(),
            )
        })
        .clone()
}

pub fn street_suffixes_fst() -> KeyedFst {
    STREET_SUFFIXES_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/lp_street_suffixes.fst").to_vec())
                    .unwrap(),
            )
        })
        .clone()
}

pub fn localities_fst() -> KeyedFst {
    LOCALITIES_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/wof_localities.fst").to_vec()).unwrap(),
            )
        })
        .clone()
}

pub fn sublocality_fst() -> KeyedFst {
    SUBLOCALITY_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/sublocality.fst").to_vec()).unwrap(),
            )
        })
        .clone()
}

pub fn regions_fst() -> KeyedFst {
    REGIONS_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/wof_regions.fst").to_vec()).unwrap(),
            )
        })
        .clone()
}

pub fn countries_fst() -> KeyedFst {
    COUNTRIES_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/wof_countries.fst").to_vec()).unwrap(),
            )
        })
        .clone()
}

pub fn intersection_join_words_fst() -> KeyedFst {
    INTERSECTION_JOIN_WORDS_FST
        .get_or_init(|| {
            KeyedFst::new(
                fst::Set::new(include_bytes!("../dicts/en/intersection_join.fst").to_vec())
                    .unwrap(),
            )
        })
        .clone()
}

pub fn brick_and_mortar_words() -> &'static HashSet<String> {
    BRICK_AND_MORTAR_WORDS.get_or_init(|| {
        fst::Set::new(include_bytes!("../dicts/en/brick_and_mortar.fst").to_vec())
            .unwrap()
            .into_stream()
            .into_strs()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    })
}
