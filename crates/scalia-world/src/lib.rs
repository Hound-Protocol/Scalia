//! World objects. One catalog (`data/catalog.ron`) says what each kind of object is: its shape, what it is made
//! of, the states it can be in (a door: open, closed, locked, breached) and the places people use it (where a man
//! sits on a bench, lies in a bunk, stands at a locker). Maps (`data/*.ron`) place objects from the catalog, plus
//! named areas and prefab buildings. Everything else (walking, sight, cover, the AI's rooms, the renderer)
//! reads the world from here instead of keeping its own copy. Integer millimetres throughout.

use scalia_core::killhouse::Killhouse;
use scalia_core::nav::{Furnish, Prop, PropKind};
use serde::Deserialize;
use std::sync::OnceLock;

/// What kind of thing an object is (for the structure scan, the editor and the AI).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Class {
    Structure,
    Cover,
    Nature,
    Furniture,
    Door,
    Hatch,
}

/// What it is made of: how bullets and sound go through it, how it is drawn.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Default)]
pub enum Material {
    #[default]
    Concrete,
    Plywood,
    Wood,
    Steel,
    Sand,
    Stone,
}

/// Its solid shape. `size` (catalog default, or the placement's own) is (along, across) before turning: a
/// wall's length and thickness, a crate's side (twice), a rock's radius (twice), furniture's width and depth, a
/// door's width and leaf thickness.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Shape {
    Wall,
    Panel { window: bool },
    Crate,
    Rock,
    Furniture(Item),
    Door { height_mm: i64 },
    Hatch,
}

/// Furniture items (scalia_core::nav::Furnish).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Item {
    Bunk,
    Locker,
    Table,
    Bench,
    Rack,
    Jukebox,
}

impl Item {
    fn furnish(self) -> Furnish {
        match self {
            Item::Bunk => Furnish::Bunk,
            Item::Locker => Furnish::Locker,
            Item::Table => Furnish::Table,
            Item::Bench => Furnish::Bench,
            Item::Rack => Furnish::Rack,
            Item::Jukebox => Furnish::Jukebox,
        }
    }
}

/// A door's (or hatch's) state.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Default)]
pub enum State {
    Open,
    #[default]
    Closed,
    /// Closed and cannot be opened (only breached).
    Locked,
    /// Forced: open for good, the leaf hanging broken.
    Breached,
}

impl State {
    /// Whether it bars the way and the view.
    pub fn shut(self) -> bool {
        matches!(self, State::Closed | State::Locked)
    }
}

/// What a man does at a place on an object (the animation to play there).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Use {
    /// Stands there (at ease, waiting).
    Stand,
    Sit,
    Lie,
    /// Works at it: a locker, a rack, a desk.
    Work,
    /// Where he stands to open or close it.
    Handle,
}

/// A place on an object, in its own frame (before turning): where, which way he faces, at what height (a top
/// bunk).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename = "Spot")]
pub struct SpotDef {
    #[serde(rename = "use")]
    pub what: Use,
    pub at: (i64, i64),
    pub face: (i64, i64),
    #[serde(default)]
    pub height_mm: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Kind {
    pub id: String,
    pub name: String,
    pub class: Class,
    pub shape: Shape,
    #[serde(default)]
    pub size: (i64, i64),
    #[serde(default)]
    pub material: Material,
    /// The states it can be in (doors, hatches); empty: none, it just stands there.
    #[serde(default)]
    pub states: Vec<State>,
    #[serde(default)]
    pub spots: Vec<SpotDef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Catalog {
    pub kinds: Vec<Kind>,
}

impl Catalog {
    pub fn parse(ron_text: &str) -> Result<Catalog, String> {
        let c: Catalog = ron::from_str(ron_text).map_err(|e| format!("catalog: {e}"))?;
        for (i, k) in c.kinds.iter().enumerate() {
            if c.kinds[..i].iter().any(|o| o.id == k.id) {
                return Err(format!("catalog: kind '{}' twice", k.id));
            }
            if matches!(k.shape, Shape::Door { .. } | Shape::Hatch) && k.states.is_empty() {
                return Err(format!("catalog: '{}' opens but lists no states", k.id));
            }
        }
        Ok(c)
    }

    pub fn find(&self, id: &str) -> Option<usize> {
        self.kinds.iter().position(|k| k.id == id)
    }
}

/// An object placed on a map.
#[derive(Clone, Debug, Deserialize)]
pub struct Placed {
    pub kind: String,
    pub at: (i64, i64),
    #[serde(default)]
    pub turn: u8,
    #[serde(default)]
    pub size: Option<(i64, i64)>,
    #[serde(default)]
    pub state: Option<State>,
}

/// A prefab building placed on a map (its walls, rooms and doors come from its own layout).
#[derive(Clone, Debug, Deserialize)]
pub struct BuildingRef {
    pub kind: String,
    pub at: (i64, i64),
    /// The state its doors start in.
    #[serde(default)]
    pub doors: State,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MapFile {
    pub name: String,
    /// The playable ground (x0, y0, x1, y1).
    pub bounds: (i64, i64, i64, i64),
    /// Named rectangles (x0, y0, x1, y1): the range, the live-fire zone, the barracks and its rooms...
    #[serde(default)]
    pub areas: Vec<(String, (i64, i64, i64, i64))>,
    /// Roof footprints for acoustic propagation (x0, y0, x1, y1, height mm).
    #[serde(default)]
    pub acoustic_roofs: Vec<AcousticRoof>,
    #[serde(default)]
    pub buildings: Vec<BuildingRef>,
    pub objects: Vec<Placed>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AcousticRoof {
    pub bounds: (i64, i64, i64, i64),
    pub height_mm: i64,
}

/// An object in the world, its kind resolved.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub kind: usize,
    pub at: (i64, i64),
    pub turn: u8,
    pub size: (i64, i64),
    /// Its state, for things that have one (doors).
    pub state: Option<State>,
}

/// A door as the simulation sees it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DoorDef {
    /// Its index among the world's objects.
    pub object: usize,
    pub at: (i64, i64),
    /// Whether the wall it hangs in runs along x.
    pub along_x: bool,
    pub width_mm: i64,
    pub height_mm: i64,
    pub material: Material,
    pub state: State,
    /// Whether it can be locked (a kind that lists Locked).
    pub lockable: bool,
}

/// A place in the world where a man does something (absolute).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Spot {
    /// The object it belongs to.
    pub object: usize,
    pub what: Use,
    pub at: (i64, i64),
    pub face: (i64, i64),
    pub height_mm: i64,
}

/// Turns a local offset by quarter turns (counter-clockwise: +x goes to +y).
pub fn turned(v: (i64, i64), turn: u8) -> (i64, i64) {
    match turn % 4 {
        0 => v,
        1 => (-v.1, v.0),
        2 => (-v.0, -v.1),
        _ => (v.1, -v.0),
    }
}

pub struct World {
    pub catalog: Catalog,
    pub map: MapFile,
    pub objects: Vec<Object>,
}

impl World {
    pub fn load(catalog: &str, map: &str) -> Result<World, String> {
        let catalog = Catalog::parse(catalog)?;
        let map: MapFile = ron::from_str(map).map_err(|e| format!("map: {e}"))?;
        for roof in &map.acoustic_roofs {
            let (x0, y0, x1, y1) = roof.bounds;
            if x0 >= x1 || y0 >= y1 || roof.height_mm <= 0 {
                return Err(format!("map '{}': invalid acoustic roof {roof:?}", map.name));
            }
        }
        let mut objects = Vec::new();
        let kind_of = |id: &str| catalog.find(id).ok_or_else(|| format!("map '{}': no kind '{id}' in the catalog", map.name));
        for b in &map.buildings {
            match b.kind.as_str() {
                "killhouse" => {
                    let k = Killhouse::new(b.at.0, b.at.1);
                    for p in k.props() {
                        let PropKind::Panel { length_mm, window } = p.kind else { continue };
                        let id = if window { "panel_window" } else { "panel" };
                        objects.push(Object { kind: kind_of(id)?, at: (p.x_mm, p.y_mm), turn: p.quarter_turns, size: (length_mm, PropKind::PANEL_MM), state: None });
                    }
                    let door = kind_of("door_wood")?;
                    for d in &k.doors {
                        let thick = catalog.kinds[door].size.1;
                        objects.push(Object { kind: door, at: (d.x_mm, d.y_mm), turn: if d.along_x { 0 } else { 1 }, size: (d.width_mm, thick), state: Some(b.doors) });
                    }
                }
                other => return Err(format!("map '{}': no building '{other}'", map.name)),
            }
        }
        for p in &map.objects {
            let kind = kind_of(&p.kind)?;
            let k = &catalog.kinds[kind];
            let state = if k.states.is_empty() {
                None
            } else {
                let s = p.state.unwrap_or(k.states[0]);
                if !k.states.contains(&s) {
                    return Err(format!("map '{}': '{}' cannot be {s:?}", map.name, k.id));
                }
                Some(s)
            };
            objects.push(Object { kind, at: p.at, turn: p.turn, size: p.size.unwrap_or(k.size), state });
        }
        Ok(World { catalog, map, objects })
    }

    /// The testing ground (the base, the live-fire range, the training house, the barracks), loaded once.
    pub fn testing_ground() -> &'static World {
        static W: OnceLock<World> = OnceLock::new();
        W.get_or_init(|| World::load(include_str!("../data/catalog.ron"), include_str!("../data/testing_ground.ron")).expect("the testing ground loads"))
    }

    pub fn kind(&self, o: &Object) -> &Kind {
        &self.catalog.kinds[o.kind]
    }

    /// A named area (x0, y0, x1, y1).
    pub fn area(&self, name: &str) -> Option<(i64, i64, i64, i64)> {
        self.map.areas.iter().find(|(n, _)| n == name).map(|&(_, r)| r)
    }

    /// The roof surfaces that block sound traveling over walls.
    pub fn acoustic_roofs(&self) -> Vec<AcousticRoof> {
        self.map.acoustic_roofs.clone()
    }

    /// The training houses on the map (their rooms and doorways for the AI).
    pub fn killhouses(&self) -> Vec<Killhouse> {
        self.map.buildings.iter().filter(|b| b.kind == "killhouse").map(|b| Killhouse::new(b.at.0, b.at.1)).collect()
    }

    /// The solid shape of one object as the grids see it (None: nothing solid; doors are kept apart, see
    /// `doors`).
    pub fn prop_of(&self, o: &Object) -> Option<Prop> {
        let (a, b) = o.size;
        let kind = match self.kind(o).shape {
            Shape::Wall => PropKind::Wall { length_mm: a, thickness_mm: b },
            Shape::Panel { window } => PropKind::Panel { length_mm: a, window },
            Shape::Crate => PropKind::Crate { size_mm: a },
            Shape::Rock => PropKind::Rock { radius_mm: a },
            Shape::Furniture(item) => PropKind::Furniture { item: item.furnish(), width_mm: a, depth_mm: b },
            Shape::Door { .. } | Shape::Hatch => return None,
        };
        Some(Prop { kind, x_mm: o.at.0, y_mm: o.at.1, quarter_turns: o.turn })
    }

    /// Everything solid that does not move or open, in map order (doors are not in it: see `doors`).
    pub fn props(&self) -> Vec<Prop> {
        self.objects.iter().filter_map(|o| self.prop_of(o)).collect()
    }

    /// The doors, in map order, as they start.
    pub fn doors(&self) -> Vec<DoorDef> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| {
                let k = self.kind(o);
                let Shape::Door { height_mm } = k.shape else { return None };
                Some(DoorDef { object: i, at: o.at, along_x: o.turn % 2 == 0, width_mm: o.size.0, height_mm, material: k.material, state: o.state.unwrap_or_default(), lockable: k.states.contains(&State::Locked) })
            })
            .collect()
    }

    /// Every place where a man does something, in map order.
    pub fn spots(&self) -> Vec<Spot> {
        let mut out = Vec::new();
        for (i, o) in self.objects.iter().enumerate() {
            for s in &self.kind(o).spots {
                let d = turned(s.at, o.turn);
                out.push(Spot { object: i, what: s.what, at: (o.at.0 + d.0, o.at.1 + d.1), face: turned(s.face, o.turn), height_mm: s.height_mm });
            }
        }
        out
    }

    /// The objects of one kind (by id), with their indices.
    pub fn of_kind<'a>(&'a self, id: &'a str) -> impl Iterator<Item = (usize, &'a Object)> + 'a {
        let k = self.catalog.find(id);
        self.objects.iter().enumerate().filter(move |(_, o)| Some(o.kind) == k)
    }
}
