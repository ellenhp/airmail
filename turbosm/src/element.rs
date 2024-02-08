use std::{collections::HashMap, error::Error};

use s2::{cellid::CellID, latlng::LatLng};

use crate::Turbosm;

#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) id: u64,
    pub(crate) lat: f64,
    pub(crate) lng: f64,
    pub(crate) tags: HashMap<String, String>,
}

impl Node {
    pub(crate) fn from_bytes(
        id: u64,
        bytes: &[u8],
        turbosm: &Turbosm,
    ) -> Result<Node, Box<dyn Error>> {
        let s2cell = u64::from_le_bytes(bytes[0..8].try_into()?);
        let mut cursor = 8usize;
        let mut tags = HashMap::new();
        loop {
            if cursor >= bytes.len() {
                break;
            }
            let key_hash = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?);
            let val_hash = u64::from_le_bytes(bytes[cursor + 8..cursor + 16].try_into()?);
            let key = turbosm.keys.get(&key_hash, turbosm);
            let val = turbosm.values.get(&val_hash, turbosm);
            if key.is_none() || val.is_none() {
                return Err("key or value not found".into());
            }
            tags.insert(
                String::from_utf8_lossy(&key.unwrap()).to_string(),
                String::from_utf8_lossy(&val.unwrap()).to_string(),
            );
            cursor += 16;
        }
        let s2cell = CellID(s2cell);
        let latlng = LatLng::from(s2cell);
        Ok(Node {
            id,
            lat: latlng.lat.deg(),
            lng: latlng.lng.deg(),
            tags,
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn lat(&self) -> f64 {
        self.lat
    }

    pub fn lng(&self) -> f64 {
        self.lng
    }

    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}

#[derive(Debug, Clone)]
pub struct Way {
    pub(crate) id: u64,
    pub(crate) nodes: Vec<Node>,
    pub(crate) tags: HashMap<String, String>,
}

impl Way {
    pub(crate) fn from_bytes(
        id: u64,
        bytes: &[u8],
        turbosm: &Turbosm,
    ) -> Result<Way, Box<dyn Error>> {
        let mut nodes = Vec::new();
        let node_count = u64::from_le_bytes(bytes[0..8].try_into()?);
        let mut cursor = 8usize;
        for _ in 0..node_count {
            let node = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?);
            nodes.push(turbosm.node(node)?);
            cursor += 8;
        }
        let mut tags = HashMap::new();
        loop {
            if cursor >= bytes.len() {
                break;
            }
            let key_hash = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?);
            let val_hash = u64::from_le_bytes(bytes[cursor + 8..cursor + 16].try_into()?);
            let key = turbosm.keys.get(&key_hash, turbosm);
            let val = turbosm.values.get(&val_hash, turbosm);
            if key.is_none() || val.is_none() {
                return Err("key or value not found".into());
            }
            tags.insert(
                String::from_utf8_lossy(&key.unwrap()).to_string(),
                String::from_utf8_lossy(&val.unwrap()).to_string(),
            );
            cursor += 16;
        }
        Ok(Way { id, nodes, tags })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}

#[derive(Debug, Clone)]
pub enum RelationMember {
    Node(String, Node),
    Way(String, Way),
    Relation(String, Box<Relation>),
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub(crate) id: u64,
    pub(crate) members: Vec<RelationMember>,
    pub(crate) tags: HashMap<String, String>,
}

impl Relation {
    pub(crate) fn from_bytes(
        id: u64,
        bytes: &[u8],
        turbosm: &Turbosm,
    ) -> Result<Relation, Box<dyn Error>> {
        let mut members = Vec::new();
        let member_count = u64::from_le_bytes(bytes[0..8].try_into()?);
        let mut cursor = 8usize;
        for _ in 0..member_count {
            let role_hash = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?);
            let role = turbosm.roles.get(&role_hash, turbosm);
            if role.is_none() {
                return Err("role not found".into());
            }
            let role = String::from_utf8_lossy(&role.unwrap()).to_string();
            cursor += 8;
            let member_type = bytes[cursor];
            cursor += 1;
            let member_id = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?);
            cursor += 8;
            let member = match member_type {
                0 => RelationMember::Node(role, turbosm.node(member_id)?),
                1 => RelationMember::Way(role, turbosm.way(member_id)?),
                2 => RelationMember::Relation(role, Box::new(turbosm.relation(member_id)?)),
                _ => return Err("invalid member type".into()),
            };
            members.push(member);
        }
        let mut tags = HashMap::new();
        loop {
            if cursor >= bytes.len() {
                break;
            }
            let key_hash = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?);
            let val_hash = u64::from_le_bytes(bytes[cursor + 8..cursor + 16].try_into()?);
            let key = turbosm.keys.get(&key_hash, turbosm);
            let val = turbosm.values.get(&val_hash, turbosm);
            if key.is_none() || val.is_none() {
                return Err("key or value not found".into());
            }
            tags.insert(
                String::from_utf8_lossy(&key.unwrap()).to_string(),
                String::from_utf8_lossy(&val.unwrap()).to_string(),
            );
            cursor += 16;
        }
        Ok(Relation { id, members, tags })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn members(&self) -> &[RelationMember] {
        &self.members
    }

    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}
