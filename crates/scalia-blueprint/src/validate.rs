//! Blueprint validation. All geometry is checked in i64 so nothing can wrap.
//!
//! Frame: the hull is a box centred on the origin in x (width) and y (length),
//! with z from 0 to `height_mm`. Boxes, wedges and rooms give their minimum
//! corner; a cylinder gives the centre of its base.

use crate::types::{Blueprint, Class, Prim};
use crate::BlueprintError;

pub fn triangle_budget(c: Class) -> u32 {
    match c {
        Class::Walker => 400,
        Class::Tank | Class::Truck | Class::Transport => 1200,
        Class::Ship => 4000,
    }
}

pub fn triangles_of(p: &Prim) -> u32 {
    match p {
        Prim::Box { .. } => 12,
        Prim::Wedge { .. } => 8,
        Prim::Cylinder { .. } => 28,
    }
}

pub fn triangle_count(b: &Blueprint) -> u32 {
    b.model.iter().fold(0u32, |n, p| n.saturating_add(triangles_of(p)))
}

fn err(field: impl Into<String>, message: impl Into<String>) -> BlueprintError {
    BlueprintError { field: field.into(), message: message.into() }
}

/// Is the rectangle [x, x+w] by [y, y+d] inside the hull footprint? Doubled coordinates avoid halves.
fn rect_inside(b: &Blueprint, x: i64, y: i64, w: i64, d: i64) -> bool {
    let hw = b.hull.width_mm as i64;
    let hl = b.hull.length_mm as i64;
    2 * x >= -hw && 2 * (x + w) <= hw && 2 * y >= -hl && 2 * (y + d) <= hl
}

pub fn validate(b: &Blueprint) -> Result<(), BlueprintError> {
    if b.id.is_empty() || !b.id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_') {
        return Err(err("id", "must be non-empty and use only a-z, 0-9 and _"));
    }
    if b.version == 0 {
        return Err(err("version", "must be at least 1"));
    }
    if b.hull.length_mm == 0 {
        return Err(err("hull.length_mm", "must be positive"));
    }
    if b.hull.width_mm == 0 {
        return Err(err("hull.width_mm", "must be positive"));
    }
    if b.hull.height_mm == 0 {
        return Err(err("hull.height_mm", "must be positive"));
    }
    if b.stats.passengers as u32 > b.interior.seats {
        return Err(err("stats.passengers", "more passengers than seats"));
    }
    if matches!(b.class, Class::Transport | Class::Ship) && b.interior.seat_cols == 0 {
        return Err(err("interior.seat_cols", "a transport or ship needs at least one seat column"));
    }
    if matches!(b.class, Class::Transport | Class::Ship) {
        // Boarding throughput is the narrowest of these three; any zero means nobody can board.
        for (name, v) in [
            ("interior.ramps", b.interior.ramps),
            ("interior.doors", b.interior.doors),
            ("interior.corridor_width_cm", b.interior.corridor_width_cm),
        ] {
            if v == 0 {
                return Err(err(name, "a transport or ship needs this above zero or nobody can board"));
            }
        }
    }
    for (i, r) in b.interior.rooms.iter().enumerate() {
        let f = format!("interior.rooms[{i}]");
        if r.w_mm == 0 || r.d_mm == 0 {
            return Err(err(f, "room size must be positive"));
        }
        if !rect_inside(b, r.x_mm as i64, r.y_mm as i64, r.w_mm as i64, r.d_mm as i64) {
            return Err(err(f, "room lies outside the hull"));
        }
    }
    if b.model.is_empty() {
        return Err(err("model", "needs at least one primitive"));
    }
    let hh = b.hull.height_mm as i64;
    for (i, p) in b.model.iter().enumerate() {
        let f = format!("model[{i}]");
        let (x, y, z, w, d, h) = match *p {
            Prim::Box { x, y, z, w, d, h, .. } | Prim::Wedge { x, y, z, w, d, h, .. } => {
                (x as i64, y as i64, z as i64, w as i64, d as i64, h as i64)
            }
            Prim::Cylinder { x, y, z, radius, h, .. } => {
                let r = radius as i64;
                (x as i64 - r, y as i64 - r, z as i64, 2 * r, 2 * r, h as i64)
            }
        };
        if w == 0 || d == 0 || h == 0 {
            return Err(err(f, "primitive size must be positive"));
        }
        if !rect_inside(b, x, y, w, d) || z < 0 || z + h > hh {
            return Err(err(f, "primitive lies outside the hull"));
        }
    }
    let (n, budget) = (triangle_count(b), triangle_budget(b.class));
    if n > budget {
        return Err(err("model", format!("{n} triangles exceeds the budget of {budget}")));
    }
    Ok(())
}
