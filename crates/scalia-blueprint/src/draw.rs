//! Top-down SVG drawing. All coordinates are integers: millimetres doubled, so a
//! hull centred on the origin never needs half units. +y (the back) is drawn up.

use crate::types::{Blueprint, Prim};
use std::fmt::Write;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

/// A rectangle with minimum corner (x, y) and size (w, d), as an SVG `<rect>` body.
fn rect(x: i64, y: i64, w: i64, d: i64) -> String {
    format!("x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"", 2 * x, -2 * (y + d), 2 * w, 2 * d)
}

pub fn svg(b: &Blueprint) -> String {
    let (w, l) = (b.hull.width_mm as i64, b.hull.length_mm as i64);
    let mut s = String::new();
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" data-id=\"{}\">",
        -w,
        -l,
        2 * w,
        2 * l,
        esc(&b.id)
    );
    let _ = write!(s, "<rect class=\"hull\" {} fill=\"none\" stroke=\"#444\"/>", rect(-w / 2, -l / 2, w, l));
    for p in &b.model {
        let (x, y, pw, pd, c) = match *p {
            Prim::Box { x, y, w, d, color, .. } | Prim::Wedge { x, y, w, d, color, .. } => {
                (x as i64, y as i64, w as i64, d as i64, color)
            }
            Prim::Cylinder { x, y, radius, color, .. } => {
                let r = radius as i64;
                (x as i64 - r, y as i64 - r, 2 * r, 2 * r, color)
            }
        };
        let _ = write!(s, "<rect class=\"prim\" data-color=\"{}\" {} fill=\"#9aa\" fill-opacity=\"0.5\"/>", c, rect(x, y, pw, pd));
    }
    for r in &b.interior.rooms {
        let _ = write!(
            s,
            "<rect class=\"room\" data-room=\"{}\" data-purpose=\"{:?}\" {} fill=\"none\" stroke=\"#c60\"/>",
            esc(&r.name),
            r.purpose,
            rect(r.x_mm as i64, r.y_mm as i64, r.w_mm as i64, r.d_mm as i64)
        );
    }
    s.push_str("</svg>");
    s
}
