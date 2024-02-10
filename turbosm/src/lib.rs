pub mod element;

use std::{
    collections::HashMap,
    error::Error,
    fs::OpenOptions,
    hash::{DefaultHasher, Hash, Hasher},
    io::Read,
    path::PathBuf,
    str::FromStr,
    thread,
    time::Instant,
};

use element::{Node, Relation, Way};
use log::info;
use memmap2::MmapMut;
use osmpbf::{Element, ElementReader, RelMemberType};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use s2::{cellid::CellID, latlng::LatLng};

pub struct ElementTable<'a, E> {
    cursor: &'a mut u64,
    blob_cursor: &'a mut u64,
    sorted_limit: u64,
    ids: &'a mut [(u64, u64, u64)],
    blobs: &'a mut [u8],
    constructor: fn(u64, &[u8], &Turbosm) -> Result<E, Box<dyn Error>>,
    tag_constructor: fn(&[u8], &Turbosm) -> Result<Vec<u64>, Box<dyn Error>>,
    cache: HashMap<u64, (u64, u64)>,
    indices_mmap: MmapMut,
    blobs_mmap: MmapMut,
    indices_file: std::fs::File,
    blobs_file: std::fs::File,
    iter_key_blocklist: Vec<u64>,
}

impl<'a, E> ElementTable<'a, E> {
    fn create_internal(
        cursor: *mut u64,
        blob_cursor: *mut u64,
        ids: *mut (u64, u64, u64),
        blobs: *mut u8,
        constructor: fn(u64, &[u8], &Turbosm) -> Result<E, Box<dyn Error>>,
        tag_constructor: fn(&[u8], &Turbosm) -> Result<Vec<u64>, Box<dyn Error>>,
        indices_mmap: MmapMut,
        blobs_mmap: MmapMut,
        indices_file: std::fs::File,
        blobs_file: std::fs::File,
        iter_key_blocklist: Vec<u64>,
    ) -> ElementTable<'a, E> {
        let table = ElementTable {
            cursor: unsafe { &mut *cursor },
            blob_cursor: unsafe { &mut *blob_cursor },
            sorted_limit: unsafe { *cursor },
            ids: unsafe { std::slice::from_raw_parts_mut(ids, (indices_mmap.len() - 16) / 24) },
            blobs: unsafe { std::slice::from_raw_parts_mut(blobs, blobs_mmap.len()) },
            constructor,
            tag_constructor,
            cache: Default::default(),
            indices_mmap,
            blobs_mmap,
            indices_file,
            blobs_file,
            iter_key_blocklist,
        };
        table
    }

    pub fn create(
        base_path: &str,
        initial_size: Option<usize>,
        constructor: fn(u64, &[u8], &Turbosm) -> Result<E, Box<dyn Error>>,
        tag_constructor: fn(&[u8], &Turbosm) -> Result<Vec<u64>, Box<dyn Error>>,
        iter_key_blocklist: Vec<u64>,
    ) -> Result<ElementTable<'a, E>, Box<dyn Error>> {
        let indices_path = format!("{}_indices", &base_path);
        let blob_path = format!("{}_blobs", &base_path);

        if let Some(initial_size) = initial_size {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&indices_path)?;
            file.set_len(initial_size as u64 * 24 + 16)?;
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&blob_path)?;
            file.set_len(initial_size as u64 * 8)?;
        }

        let indices_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&indices_path)?;
        let mut indices_mmap = unsafe {
            memmap2::MmapOptions::new()
                .len(indices_file.metadata()?.len() as usize)
                .map_mut(&indices_file)?
        };
        let blobs_file = OpenOptions::new().read(true).write(true).open(&blob_path)?;
        let mut blobs_mmap = unsafe {
            memmap2::MmapOptions::new()
                .len(blobs_file.metadata()?.len() as usize)
                .map_mut(&blobs_file)?
        };
        let mut table = Self::create_internal(
            indices_mmap.as_mut_ptr() as *mut u64,
            unsafe { indices_mmap.as_mut_ptr().add(8) as *mut u64 },
            unsafe { (indices_mmap.as_ptr().add(16)) as *mut (u64, u64, u64) },
            blobs_mmap.as_mut_ptr(),
            constructor,
            tag_constructor,
            indices_mmap,
            blobs_mmap,
            indices_file,
            blobs_file,
            iter_key_blocklist,
        );
        if initial_size.is_none() {
            table.sorted_limit = *table.cursor;
        }
        Ok(table)
    }

    pub fn open_ro(
        base_path: &str,
        constructor: fn(u64, &[u8], &Turbosm) -> Result<E, Box<dyn Error>>,
        tag_constructor: fn(&[u8], &Turbosm) -> Result<Vec<u64>, Box<dyn Error>>,
        iter_key_blocklist: Vec<u64>,
    ) -> Result<ElementTable<'a, E>, Box<dyn Error>> {
        Self::create(
            base_path,
            None,
            constructor,
            tag_constructor,
            iter_key_blocklist,
        )
    }

    pub fn get(&self, id: &u64, turbosm: &Turbosm) -> Option<E> {
        if let Some(blob) = self.get_raw(id) {
            let element = (self.constructor)(*id, blob, turbosm);
            if let Ok(element) = element {
                return Some(element);
            } else {
                return None;
            }
        }
        None
    }

    pub fn get_raw(&self, id: &u64) -> Option<&[u8]> {
        if self.cache.contains_key(id) {
            let (offset, len) = self.cache[id];
            return Some(&self.blobs[offset as usize..(offset + len) as usize]);
        }
        if let Ok(idx) =
            self.ids[..self.sorted_limit as usize].binary_search_by_key(&id, |(id, _, _)| id)
        {
            let (_id, offset, len) = self.ids[idx];
            return Some(&self.blobs[offset as usize..(offset + len as u64) as usize]);
        } else {
            None
        }
    }

    pub fn insert(&mut self, id: &u64, blob: &[u8]) {
        if *self.cursor >= self.ids.len() as u64 {
            let current_len = self.indices_mmap.len() as u64;
            let new_len = if current_len > 1024 * 1024 * 1024 {
                current_len + 1024 * 1024 * 1024
            } else if current_len == 0 {
                1024 * 1024
            } else {
                current_len * 2
            };
            println!(
                "Growing indices file from {} to {}",
                self.ids.len(),
                new_len
            );
            self.indices_file.set_len(16 + new_len).unwrap();
            self.indices_mmap = unsafe {
                memmap2::MmapOptions::new()
                    .len(self.indices_file.metadata().unwrap().len() as usize)
                    .huge(None)
                    .map_mut(&self.indices_file)
                    .unwrap()
            };

            self.cursor = unsafe { &mut *(self.indices_mmap.as_mut_ptr() as *mut u64) };
            self.blob_cursor = unsafe { &mut *(self.indices_mmap.as_mut_ptr().add(8) as *mut u64) };
            self.ids = unsafe {
                let ids_ptr = (self.indices_mmap.as_ptr().add(16)) as *mut (u64, u64, u64);
                std::slice::from_raw_parts_mut(ids_ptr, (self.indices_mmap.len() - 16) / 24)
            };
        }
        // This is a while loop because we might need to grow the blobs file more than once if someone inserts a huge blob.
        while *self.blob_cursor + blob.len() as u64 >= self.blobs.len() as u64 {
            let current_len = self.blobs_mmap.len() as u64;
            let new_len = if current_len > 1024 * 1024 * 1024 {
                current_len + 1024 * 1024 * 1024
            } else if current_len == 0 {
                1024 * 1024
            } else {
                current_len * 2
            };
            println!("Growing blobs file from {} to {}", current_len, new_len);
            self.blobs_file.set_len(new_len).unwrap();
            self.blobs_mmap = unsafe {
                memmap2::MmapOptions::new()
                    .len(self.blobs_file.metadata().unwrap().len() as usize)
                    .huge(None)
                    .map_mut(&self.blobs_file)
                    .unwrap()
            };
            self.blobs = unsafe {
                std::slice::from_raw_parts_mut(self.blobs_mmap.as_mut_ptr(), new_len as usize)
            };
        }

        self.ids[*self.cursor as usize] = (
            *id,
            *self.blob_cursor,
            blob.len().try_into().expect("blob too large"),
        );
        self.blobs[*self.blob_cursor as usize..*self.blob_cursor as usize + blob.len()]
            .copy_from_slice(blob);
        *self.blob_cursor += blob.len() as u64;
        *self.cursor += 1;
    }

    pub fn sort(&mut self) {
        self.ids[..*self.cursor as usize].sort_unstable_by_key(|(id, _, _)| *id);
        self.sorted_limit = *self.cursor;
        self.cache.clear();
    }

    pub fn sort_blobs(&mut self) {
        self.blobs[..*self.blob_cursor as usize].sort_unstable();
    }

    pub fn for_each<Callback: Sync + Fn(E, &Turbosm)>(
        &self,
        turbosm: &Turbosm,
        callback: Callback,
    ) {
        self.ids[..*self.cursor as usize]
            .par_iter()
            .for_each(|(id, _, _)| {
                if !self.iter_key_blocklist.is_empty() {
                    let tags = (self.tag_constructor)(&self.get_raw(id).unwrap(), turbosm).unwrap();
                    if tags.iter().any(|t| self.iter_key_blocklist.contains(t)) {
                        return;
                    }
                }
                if let Some(element) = self.get(id, turbosm) {
                    callback(element, turbosm);
                }
            });
    }
}

fn count_entities<R: Read + Send>(
    reader: ElementReader<R>,
) -> Result<(u64, u64, u64), Box<dyn Error>> {
    Ok(reader.par_map_reduce(
        |element| match element {
            Element::Node(_) => (1u64, 0u64, 0u64),
            Element::DenseNode(_) => (1u64, 0u64, 0u64),
            Element::Way(_) => (0u64, 1u64, 0u64),
            Element::Relation(_) => (0u64, 0u64, 1u64),
        },
        || (0, 0, 0),
        |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2),
    )?)
}

enum EntityId {
    Node(u64),
    Way(u64),
    Relation(u64),
}

enum PendingElement {
    Node {
        id: u64,
        s2cell: u64,
        tags: HashMap<String, String>,
    },
    Way {
        id: u64,
        tags: HashMap<String, String>,
        members: Vec<u64>,
    },
    Relation {
        id: u64,
        tags: HashMap<String, String>,
        members: Vec<(String, EntityId)>,
    },
}

pub struct Turbosm<'a> {
    nodes: ElementTable<'a, Node>,
    ways: ElementTable<'a, Way>,
    relations: ElementTable<'a, Relation>,
    keys: ElementTable<'a, Vec<u8>>,
    values: ElementTable<'a, Vec<u8>>,
    roles: ElementTable<'a, Vec<u8>>,
}

impl<'a> Turbosm<'a> {
    fn process_entity(
        &mut self,
        extra: &[u8],
        tags: &[(String, String)],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut packed = Vec::new();
        packed.extend_from_slice(&extra);
        for (key, value) in tags {
            let (key_hash, value_hash) = {
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                let key_hash = hasher.finish();
                let mut hasher = DefaultHasher::new();
                value.hash(&mut hasher);
                let value_hash = hasher.finish();
                (key_hash, value_hash)
            };
            if self.keys.get(&key_hash, self).is_none() {
                self.keys.insert(&key_hash, key.as_bytes());
            }
            if self.values.get(&value_hash, self).is_none() {
                self.values.insert(&value_hash, value.as_bytes());
            }
            packed.extend_from_slice(&key_hash.to_le_bytes());
            packed.extend_from_slice(&value_hash.to_le_bytes());
        }
        Ok(packed)
    }

    fn process_node(
        &mut self,
        id: u64,
        s2cell: u64,
        tags: &[(String, String)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let extra = s2cell.to_le_bytes();
        let packed = self.process_entity(&extra, tags)?;
        self.nodes.insert(&id, packed.as_slice());
        Ok(())
    }

    fn process_way(
        &mut self,
        id: u64,
        tags: &[(String, String)],
        members: &[u64],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut extra = Vec::new();
        extra.extend((members.len() as u64).to_le_bytes());
        for member in members {
            extra.extend(member.to_le_bytes());
        }
        let packed = self.process_entity(&extra, &tags)?;
        self.ways.insert(&id, packed.as_slice());
        Ok(())
    }

    fn process_relation(
        &mut self,
        id: u64,
        tags: &[(String, String)],
        members: &[(String, EntityId)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut extra = Vec::new();
        extra.extend((members.len() as u64).to_le_bytes());
        for (role, member) in members {
            let role_hash = {
                let mut hasher = DefaultHasher::new();
                role.hash(&mut hasher);
                hasher.finish()
            };
            if self.roles.get(&role_hash, self).is_none() {
                self.roles.insert(&role_hash, role.as_bytes());
            }
            extra.extend(role_hash.to_le_bytes());
            match member {
                EntityId::Node(id) => {
                    extra.push(0u8);
                    extra.extend(id.to_le_bytes());
                }
                EntityId::Way(id) => {
                    extra.push(1u8);
                    extra.extend(id.to_le_bytes());
                }
                EntityId::Relation(id) => {
                    extra.push(2u8);
                    extra.extend(id.to_le_bytes());
                }
            }
        }
        let packed = self.process_entity(&extra, tags)?;
        self.relations.insert(&id, packed.as_slice());
        Ok(())
    }

    pub fn create_from_pbf(
        pbf_path: &'_ str,
        db_path: &'_ str,
    ) -> Result<Turbosm<'a>, Box<dyn std::error::Error>> {
        info!("Counting entities");
        let (node_count, way_count, relation_count) =
            count_entities(ElementReader::from_path(pbf_path)?)?;
        info!(
            "Total entities: {}",
            node_count + way_count + relation_count
        );

        let nodes_path = PathBuf::from_str(db_path)?.join("nodes");
        let nodes = ElementTable::create(
            &*nodes_path.to_string_lossy(),
            Some(node_count as usize),
            Node::from_bytes,
            Node::tags_from_bytes,
            vec![],
        )?;
        let ways_path = PathBuf::from_str(db_path)?.join("ways");
        let ways = ElementTable::create(
            &*ways_path.to_string_lossy(),
            Some(way_count as usize),
            Way::from_bytes,
            Way::tags_from_bytes,
            vec![],
        )?;
        let relations_path = PathBuf::from_str(db_path)?.join("relations");
        let relations = ElementTable::create(
            &*relations_path.to_string_lossy(),
            Some(relation_count as usize),
            Relation::from_bytes,
            Relation::tags_from_bytes,
            vec![],
        )?;
        let keys_path = PathBuf::from_str(db_path)?.join("keys");
        let keys = ElementTable::create(
            &*keys_path.to_string_lossy(),
            Some(1024 * 1024),
            |_id, bytes, _| Ok(bytes.to_vec()),
            |_bytes, _| Ok(vec![]),
            vec![],
        )?;
        let values_path = PathBuf::from_str(db_path)?.join("values");
        let values = ElementTable::create(
            &*values_path.to_string_lossy(),
            Some(1024 * 1024),
            |_id, bytes, _| Ok(bytes.to_vec()),
            |_bytes, _| Ok(vec![]),
            vec![],
        )?;
        let roles_path = PathBuf::from_str(db_path)?.join("roles");
        let roles = ElementTable::create(
            &*roles_path.to_string_lossy(),
            Some(1024 * 1024),
            |_id, bytes, _| Ok(bytes.to_vec()),
            |_bytes, _| Ok(vec![]),
            vec![],
        )?;

        let mut osm = Turbosm {
            nodes: nodes,
            ways: ways,
            relations: relations,
            keys: keys,
            values: values,
            roles: roles,
        };
        info!("Loading PBF");
        osm.load_pbf(pbf_path, node_count + way_count + relation_count)?;
        info!("Done loading pbf");
        Ok(osm)
    }

    pub fn open(
        db_path: &'_ str,
        blocked_keys: &'_ [&'_ str],
    ) -> Result<Turbosm<'a>, Box<dyn std::error::Error>> {
        let blocked_keys = blocked_keys
            .iter()
            .map(|k| {
                let mut hasher = DefaultHasher::new();
                k.hash(&mut hasher);
                hasher.finish()
            })
            .collect::<Vec<_>>();
        let nodes_path = PathBuf::from_str(db_path)?.join("nodes");
        let nodes = ElementTable::open_ro(
            &*nodes_path.to_string_lossy(),
            Node::from_bytes,
            Node::tags_from_bytes,
            blocked_keys.clone(),
        )?;
        let ways_path = PathBuf::from_str(db_path)?.join("ways");
        let ways = ElementTable::open_ro(
            &*ways_path.to_string_lossy(),
            Way::from_bytes,
            Way::tags_from_bytes,
            blocked_keys.clone(),
        )?;
        let relations_path = PathBuf::from_str(db_path)?.join("relations");
        let relations = ElementTable::open_ro(
            &*relations_path.to_string_lossy(),
            Relation::from_bytes,
            Relation::tags_from_bytes,
            blocked_keys.clone(),
        )?;
        let keys_path = PathBuf::from_str(db_path)?.join("keys");
        let keys = ElementTable::open_ro(
            &*keys_path.to_string_lossy(),
            |_id, bytes, _| Ok(bytes.to_vec()),
            |_bytes, _| Ok(vec![]),
            vec![],
        )?;
        let values_path = PathBuf::from_str(db_path)?.join("values");
        let values = ElementTable::open_ro(
            &*values_path.to_string_lossy(),
            |_id, bytes, _| Ok(bytes.to_vec()),
            |_bytes, _| Ok(vec![]),
            vec![],
        )?;
        let roles_path = PathBuf::from_str(db_path)?.join("roles");
        let roles = ElementTable::open_ro(
            &*roles_path.to_string_lossy(),
            |_id, bytes, _| Ok(bytes.to_vec()),
            |_bytes, _| Ok(vec![]),
            vec![],
        )?;

        Ok(Turbosm {
            nodes,
            ways,
            relations,
            keys,
            values,
            roles,
        })
    }

    pub fn load_pbf(
        &mut self,
        pbf_path: &str,
        total_entities: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let pbf = ElementReader::from_path(pbf_path)?;

        info!("Processing PBF file");
        let (sender, receiver) = std::sync::mpsc::sync_channel(10000);
        thread::spawn(move || {
            pbf.for_each(move |element| match element {
                osmpbf::Element::Node(node) => {
                    let id = node.id() as u64;
                    let s2cell = CellID::from(LatLng::from_degrees(node.lat(), node.lon())).0;
                    sender
                        .send(PendingElement::Node {
                            id,
                            s2cell,
                            tags: node
                                .tags()
                                .into_iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                        })
                        .unwrap();
                }
                osmpbf::Element::DenseNode(node) => {
                    let id = node.id() as u64;
                    let s2cell = CellID::from(LatLng::from_degrees(node.lat(), node.lon())).0;
                    sender
                        .send(PendingElement::Node {
                            id,
                            s2cell,
                            tags: node
                                .tags()
                                .into_iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                        })
                        .unwrap();
                }
                osmpbf::Element::Way(way) => {
                    let tags: Vec<(&str, &str)> = way.tags().collect();
                    let id = way.id() as u64;
                    let members: Vec<u64> = way.refs().map(|r| r as u64).collect();
                    sender
                        .send(PendingElement::Way {
                            id,
                            tags: tags
                                .into_iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                            members,
                        })
                        .unwrap();
                }
                osmpbf::Element::Relation(relation) => {
                    let id = relation.id() as u64;
                    let members: Vec<(String, EntityId)> = relation
                        .members()
                        .map(|r| match r.member_type {
                            RelMemberType::Node => (
                                r.role().unwrap().to_string(),
                                EntityId::Node(r.member_id as u64),
                            ),
                            RelMemberType::Way => (
                                r.role().unwrap().to_string(),
                                EntityId::Way(r.member_id as u64),
                            ),
                            RelMemberType::Relation => (
                                r.role().unwrap().to_string(),
                                EntityId::Relation(r.member_id as u64),
                            ),
                        })
                        .collect();
                    sender
                        .send(PendingElement::Relation {
                            id,
                            tags: relation
                                .tags()
                                .into_iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                            members,
                        })
                        .unwrap();
                }
            })
            .unwrap();
        });
        let mut count = 0u64;

        loop {
            let element = if let Ok(val) = receiver.recv() {
                val
            } else {
                break;
            };
            count += 1;
            if count % 100000 == 0 {
                info!(
                    "Processed {} of {} ({}%) in {} seconds, {} per second",
                    count,
                    total_entities,
                    count as f64 / total_entities as f64 * 100.0,
                    start.elapsed().as_secs(),
                    count as f64 / start.elapsed().as_secs_f64()
                );
            }

            match element {
                PendingElement::Node { id, s2cell, tags } => {
                    let tags: Vec<_> = tags.into_iter().collect();
                    self.process_node(id, s2cell, &tags)?;
                }
                PendingElement::Way { id, tags, members } => {
                    let tags: Vec<_> = tags.into_iter().collect();
                    self.process_way(id, &tags, &members)?;
                }
                PendingElement::Relation { id, tags, members } => {
                    let tags: Vec<_> = tags.into_iter().collect();
                    self.process_relation(id, &tags, &members)?;
                }
            }
        }
        self.nodes.sort();
        self.ways.sort();
        self.relations.sort();
        self.keys.sort();
        self.values.sort();
        self.roles.sort();
        Ok(())
    }

    pub fn close(self) {}

    pub fn node(&self, id: u64) -> Result<Node, Box<dyn std::error::Error>> {
        self.nodes.get(&id, self).ok_or("Node not found".into())
    }

    pub fn way(&self, id: u64) -> Result<Way, Box<dyn std::error::Error>> {
        self.ways.get(&id, self).ok_or("Way not found".into())
    }

    pub fn relation(&self, id: u64) -> Result<Relation, Box<dyn std::error::Error>> {
        self.relations
            .get(&id, self)
            .ok_or("Relation not found".into())
    }

    pub fn process_all_nodes<Callback: Sync + Fn(Node, &Turbosm) -> ()>(
        &mut self,
        cb: Callback,
    ) -> Result<(), Box<dyn Error>> {
        self.nodes.for_each(self, cb);
        Ok(())
    }

    pub fn process_all_ways<Callback: Sync + Fn(Way, &Turbosm) -> ()>(
        &mut self,
        cb: Callback,
    ) -> Result<(), Box<dyn Error>> {
        self.ways.for_each(self, cb);
        Ok(())
    }

    pub fn process_all_relations<Callback: Sync + Fn(Relation, &Turbosm) -> ()>(
        &mut self,
        cb: Callback,
    ) -> Result<(), Box<dyn Error>> {
        println!("locking relations");
        self.relations.for_each(self, cb);
        Ok(())
    }
}
