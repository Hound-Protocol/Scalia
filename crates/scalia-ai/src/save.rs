//! The simulation contract: the map a battle is fought on (enough to rebuild its grids), the commands that are
//! its only input, and its saved state, a versioned and checksummed byte string whose hash is the battle's hash.
use crate::battle::{Battle, Task};
use crate::leader::Order;
use crate::sight::SightGrid;
use crate::tactics::CoverMap;
use scalia_core::hash::mix64;
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::{NavGrid, Prop, PropKind};
use serde::{Deserialize, Serialize};

/// Everything that can change a battle from outside: a player's or a leader's order.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// A soldier is given a task directly.
    Task { soldier: usize, task: Task },
    /// A squad is given the commander's intent; its leader plans from it.
    Squad { squad: usize, order: Order },
    /// A soldier joins the battle (a squad arriving on this ground).
    Spawn { team: u8, squad: u16, pos: (i64, i64), stance: crate::battle::Stance },
}

/// The ground a battle is fought on: its area (mm), its props, and the training house if there is one.
/// The navigation, sight and cover grids are rebuilt from this, so a save holds the map, not the grids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSpec {
    pub x0: i64,
    pub y0: i64,
    pub w_mm: i64,
    pub h_mm: i64,
    props: Vec<PropSpec>,
    pub killhouse: Option<(i64, i64)>,
    /// How many of the props (the last ones) were placed during the battle, in order.
    placed: usize,
    /// The live-fire zone (x0, y0, x1, y1; mm): only soldiers inside it see and fight each other. None: the
    /// whole map is live.
    pub live: Option<(i64, i64, i64, i64)>,
}

/// Navigation cells are 500 mm, sight cells 250 mm.
const NAV_CELL: i64 = 500;
const SIGHT_CELL: i64 = 250;

impl MapSpec {
    pub fn new(x0: i64, y0: i64, w_mm: i64, h_mm: i64, props: &[Prop]) -> MapSpec {
        MapSpec { x0, y0, w_mm, h_mm, props: props.iter().map(|&p| PropSpec::from(p)).collect(), killhouse: None, placed: 0, live: None }
    }

    /// The training house with its south-west corner at `origin`, on 100 × 80 m of open ground around it.
    pub fn killhouse(origin: (i64, i64)) -> MapSpec {
        let k = Killhouse::new(origin.0, origin.1);
        MapSpec { killhouse: Some(origin), ..MapSpec::new(origin.0 - 40_000, origin.1 - 42_000, 100_000, 80_000, &k.props()) }
    }

    /// Adds a prop placed during the battle (sandbags): it is part of the map from now on, saves included.
    pub fn push_prop(&mut self, p: Prop) {
        self.props.push(PropSpec::from(p));
        self.placed += 1;
    }

    pub fn props(&self) -> Vec<Prop> {
        self.props.iter().map(|&p| p.into()).collect()
    }

    /// Builds the navigation, sight and cover grids.
    pub(crate) fn grids(&self) -> (NavGrid, SightGrid, CoverMap) {
        let props = self.props();
        let base = props.len() - self.placed;
        let mut nav = NavGrid::new(self.x0, self.y0, (self.w_mm / NAV_CELL) as usize, (self.h_mm / NAV_CELL) as usize);
        nav.rebuild(&props[..base]);
        let sight = SightGrid::build(self.x0, self.y0, (self.w_mm / SIGHT_CELL) as usize, (self.h_mm / SIGHT_CELL) as usize, &props);
        let mut cover = CoverMap::build(&props[..base], &nav);
        // Props placed during the battle, replayed in order exactly as they were added (cover indices included).
        for k in base..props.len() {
            nav.rebuild(&props[..=k]);
            cover.add_prop(k, &props[k], &nav);
        }
        (nav, sight, cover)
    }
}

/// `scalia_core::nav::Prop` as saved (the core crate stays free of dependencies).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PropSpec {
    kind: u8,
    a: i64,
    b: i64,
    x_mm: i64,
    y_mm: i64,
    quarter_turns: u8,
}

impl From<Prop> for PropSpec {
    fn from(p: Prop) -> PropSpec {
        let (kind, a, b) = match p.kind {
            PropKind::Rock { radius_mm } => (0, radius_mm, 0),
            PropKind::Wall { length_mm, thickness_mm } => (1, length_mm, thickness_mm),
            PropKind::Crate { size_mm } => (2, size_mm, 0),
            PropKind::Panel { length_mm, window } => (3, length_mm, window as i64),
        };
        PropSpec { kind, a, b, x_mm: p.x_mm, y_mm: p.y_mm, quarter_turns: p.quarter_turns }
    }
}

impl From<PropSpec> for Prop {
    fn from(p: PropSpec) -> Prop {
        let kind = match p.kind {
            0 => PropKind::Rock { radius_mm: p.a },
            1 => PropKind::Wall { length_mm: p.a, thickness_mm: p.b },
            2 => PropKind::Crate { size_mm: p.a },
            _ => PropKind::Panel { length_mm: p.a, window: p.b != 0 },
        };
        Prop { kind, x_mm: p.x_mm, y_mm: p.y_mm, quarter_turns: p.quarter_turns }
    }
}

/// "SCAB" (Scalia battle) and the format version: a save from another version is refused, not misread.
const MAGIC: [u8; 4] = *b"SCAB";
pub const SAVE_VERSION: u32 = 10;

/// Frames a state as magic, version, payload and a checksum over the payload.
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 16);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&SAVE_VERSION.to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&checksum(payload).to_le_bytes());
    out
}

/// The payload of a framed save, if it is whole and of this version.
pub(crate) fn unframe(bytes: &[u8]) -> Result<&[u8], String> {
    if bytes.len() < 16 || bytes[..4] != MAGIC {
        return Err("not a Scalia battle save".into());
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != SAVE_VERSION {
        return Err(format!("save format {version}, this build reads {SAVE_VERSION}"));
    }
    let (payload, sum) = bytes[8..].split_at(bytes.len() - 16);
    if checksum(payload).to_le_bytes() != sum {
        return Err("save is damaged (checksum mismatch)".into());
    }
    Ok(payload)
}

/// A 64-bit hash of bytes (also the battle hash: the hash of its saved state).
pub fn checksum(bytes: &[u8]) -> u64 {
    let mut h = mix64(bytes.len() as u64);
    for chunk in bytes.chunks(8) {
        let mut w = [0u8; 8];
        w[..chunk.len()].copy_from_slice(chunk);
        h = mix64(h ^ u64::from_le_bytes(w));
    }
    h
}

pub(crate) fn encode<T: Serialize>(v: &T) -> Vec<u8> {
    postcard::to_allocvec(v).expect("battle state always encodes")
}

pub(crate) fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
    postcard::from_bytes(bytes).map_err(|e| format!("save does not decode: {e}"))
}

impl Battle {
    /// The battle's hash: of everything a save holds (so of everything that decides what happens next).
    pub fn hash(&self) -> u64 {
        checksum(&self.state_bytes())
    }
}
