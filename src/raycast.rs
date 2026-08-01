// raycast.rs — lanzamiento de rayos e impacto

use crate::mapa::{char_en, es_pared};

pub const PASO_RAYO: f32 = 0.02;
pub const MAX_DIST: f32 = 40.0;

pub struct Impacto {
    pub d: f32,
    pub ch: char,
    pub tx: f32,
}

pub fn lanzar(grid: &[Vec<char>], x: f32, y: f32, ang: f32) -> Impacto {
    let (dx, dy) = (ang.cos(), ang.sin());
    let mut t = 0.0f32;
    let (mut cx_ant, mut cy_ant) = (x.floor(), y.floor());

    while t < MAX_DIST {
        t += PASO_RAYO;
        let hx = x + dx * t;
        let hy = y + dy * t;
        let ch = char_en(grid, hx, hy);

        if es_pared(ch) {
            let (cx, cy) = (hx.floor(), hy.floor());
            let vertical = cx != cx_ant;

            let mut tx = if vertical {
                hy - hy.floor()
            } else if cy != cy_ant {
                hx - hx.floor()
            } else {
                hx - hx.floor()
            };

            if (vertical && dx > 0.0) || (!vertical && dy < 0.0) {
                tx = 1.0 - tx;
            }
            return Impacto { d: t, ch, tx };
        }

        cx_ant = hx.floor();
        cy_ant = hy.floor();
    }
    Impacto {
        d: MAX_DIST,
        ch: '+',
        tx: 0.0,
    }
}
