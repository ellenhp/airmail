mod importer;
mod query_pip;
pub use importer::{Importer, ImporterBuilder};

use airmail::poi::ToIndexPoi;
use crossbeam::channel::Sender;
use lingua::{IsoCode639_3, Language};
use redb::{ReadTransaction, TableDefinition};
use reqwest::Url;
use std::{error::Error, str::FromStr};

pub(crate) const TABLE_AREAS: TableDefinition<u64, &[u8]> = TableDefinition::new("admin_areas");
pub(crate) const TABLE_NAMES: TableDefinition<u64, &str> = TableDefinition::new("admin_names");
pub(crate) const TABLE_LANGS: TableDefinition<u64, &str> = TableDefinition::new("admin_langs");

const COUNTRIES: [u64; 214] = [
    85632343, 85632573, 85632229, 85632529, 85632405, 85632773, 85632281, 85632715, 85632505,
    85632785, 85632793, 85632295, 85632717, 85632609, 85632491, 85632475, 85632997, 85632213,
    85633001, 85632167, 85632285, 85632247, 85632749, 85632623, 85633009, 85632339, 85632171,
    85632235, 85632395, 85632571, 85633041, 85632643, 85632391, 85632541, 85633051, 85632449,
    85633057, 85632245, 85632695, 85632519, 85632487, 85632675, 85632721, 85632441, 85632437,
    85633105, 85633111, 85632319, 85633121, 85632503, 85632713, 85632451, 85632261, 85633135,
    85632581, 85632217, 85632781, 85633129, 85632257, 85633143, 85632755, 85632431, 85633147,
    85632407, 85633159, 85632335, 85633163, 85632547, 85632189, 85633217, 85632603, 85632691,
    85632287, 85633171, 85632385, 85632757, 85632397, 85632483, 85632323, 85633229, 85632433,
    85633237, 85632203, 85633241, 85632315, 85632461, 85632469, 85632191, 85632361, 85633249,
    85633253, 85632593, 85632215, 85632425, 85632429, 85632329, 85632761, 85632359, 85632709,
    85632259, 85632551, 85632639, 85632231, 85632401, 85632307, 85632241, 85632533, 85632369,
    85633267, 85632313, 85632249, 85632173, 85633269, 85633275, 85633279, 85632627, 85632693,
    85633285, 85633287, 85632667, 85632223, 85632663, 85632373, 85632553, 85632181, 85632439,
    85632161, 85632679, 85633331, 85632357, 85632305, 85632383, 85633293, 85632739, 85632729,
    85632535, 85632269, 85632735, 85632599, 85633337, 85633341, 85632465, 85632747, 85633345,
    85632207, 85632179, 85632521, 85632347, 85632509, 85632659, 85633723, 85633739, 85633735,
    85632331, 85632355, 85632299, 85633745, 85633755, 85632685, 85632303, 85632253, 85632591,
    85632661, 85632751, 85633789, 85632605, 85633779, 85633769, 85632467, 85633763, 85632365,
    85632379, 85632443, 85632657, 85632765, 85632545, 85632185, 85632413, 85632635, 85632325,
    85632647, 85632293, 85632513, 85632583, 85632671, 85632703, 85632455, 85632393, 85632271,
    85632607, 85632403, 85632227, 85633805, 85632625, 102312305, 85633793, 85632511, 85632645,
    85632187, 85632569, 85632317, 85632763, 85632263, 85632681, 85633259, 1, 85632733, 1729945891,
    1729945893, 1729989201, 85632499, 85633813, 85632559, 85632243,
];

pub(crate) enum WofCacheItem {
    Names(u64, Vec<String>),
    Langs(u64, Vec<String>),
    Admins(u64, Vec<u64>),
}

pub(crate) async fn populate_admin_areas<'a>(
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    poi: &mut ToIndexPoi,
    spatial_url: &Url,
) -> Result<(), Box<dyn Error>> {
    let pip_response = query_pip::query_pip(read, to_cache_sender, poi.s2cell, spatial_url).await?;
    for admin in pip_response.admin_names {
        poi.admins.push(admin);
    }
    for lang in pip_response.admin_langs {
        let _ = IsoCode639_3::from_str(&lang)
            .map(|iso| poi.languages.push(Language::from_iso_code_639_3(&iso)));
    }

    Ok(())
}
