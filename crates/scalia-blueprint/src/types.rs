//! Blueprint data model. Every number is an integer: mm, kg, mm/s.

use serde::Deserialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Class {
    Walker,
    Tank,
    Truck,
    Transport,
    Ship,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hull {
    pub length_mm: u32,
    pub width_mm: u32,
    pub height_mm: u32,
    pub mass_kg: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub speed_mm_s: u32,
    pub crew: u16,
    pub passengers: u16,
    pub armour: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Purpose {
    Bridge,
    Cargo,
    Troops,
    Engine,
    Crew,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Room {
    pub name: String,
    pub purpose: Purpose,
    pub x_mm: i32,
    pub y_mm: i32,
    pub w_mm: u32,
    pub d_mm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interior {
    pub ramps: u16,
    pub doors: u16,
    pub corridor_width_cm: u16,
    pub seat_cols: u16,
    pub seats: u32,
    pub rooms: Vec<Room>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Prim {
    Box { x: i32, y: i32, z: i32, w: u32, d: u32, h: u32, color: u8 },
    Wedge { x: i32, y: i32, z: i32, w: u32, d: u32, h: u32, color: u8 },
    Cylinder { x: i32, y: i32, z: i32, radius: u32, h: u32, color: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub class: Class,
    pub hull: Hull,
    pub stats: Stats,
    pub interior: Interior,
    pub model: Vec<Prim>,
}
