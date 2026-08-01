// render.rs — dibujo del mundo, sprites y minimapa

use raylib::prelude::*;
use crate::estado::Estado;
use crate::raycast::{lanzar, MAX_DIST};
use crate::mapa::es_pared;
use crate::{
    ANCHO, VIEW_H,
    PARED, PARED2, PARED3, PARED_BORDE, CAMINO, INICIO, META, JUGADOR,
    RAYO, RAYO_BORDE, CIELO, PISO, ENEMIGO,
};

const FOV: f32 = std::f32::consts::PI / 3.0;
const ANCHO_ESTACA: i32 = 2;
const NUM_RAYOS_2D: usize = 80;

fn color_de(ch: char) -> Color {
    match ch {
        '#' | '+' | '-' | '|' => PARED,
        'W' => PARED2,
        'X' => PARED3,
        'A' => INICIO,
        'B' => META,
        _ => CAMINO,
    }
}

fn indice_textura(ch: char) -> usize {
    match ch {
        '#' | '+' | '-' | '|' => 0,
        'W' => 1,
        'X' => 2,
        _ => 0,
    }
}

fn sombrear(c: Color, f: f32) -> Color {
    let f = f.clamp(0.0, 1.0);
    Color {
        r: (c.r as f32 * f) as u8,
        g: (c.g as f32 * f) as u8,
        b: (c.b as f32 * f) as u8,
        a: 255,
    }
}

pub fn render_2d(dh: &mut RaylibDrawHandle<'_>, est: &Estado) {
    let bs = ((ANCHO / est.cols()).min(VIEW_H / est.filas())).max(3);
    let ox = (ANCHO - est.cols() * bs) / 2;
    let oy = (VIEW_H - est.filas() * bs) / 2;

    for (r, fila) in est.grid.iter().enumerate() {
        for (c, ch) in fila.iter().enumerate() {
            let x = ox + c as i32 * bs;
            let y = oy + r as i32 * bs;
            dh.draw_rectangle(x, y, bs, bs, color_de(*ch));
            if es_pared(*ch) && bs >= 10 {
                dh.draw_rectangle(x, y, bs, 2, PARED_BORDE);
            }
        }
    }

    let px = ox as f32 + est.x * bs as f32;
    let py = oy as f32 + est.y * bs as f32;

    for i in 0..NUM_RAYOS_2D {
        let t = i as f32 / (NUM_RAYOS_2D - 1) as f32;
        let ang = est.a - FOV / 2.0 + FOV * t;
        let imp = lanzar(&est.grid, est.x, est.y, ang);
        let fin = Vector2::new(
            px + ang.cos() * imp.d * bs as f32,
            py + ang.sin() * imp.d * bs as f32,
        );
        let borde = i == 0 || i == NUM_RAYOS_2D - 1;
        let (grosor, color) = if borde { (2.0, RAYO_BORDE) } else { (1.0, RAYO) };
        dh.draw_line_ex(Vector2::new(px, py), fin, grosor, color);
    }

    let ex = ox as f32 + est.ex * bs as f32;
    let ey = oy as f32 + est.ey * bs as f32;
    dh.draw_circle_v(Vector2::new(ex, ey), bs as f32 * 0.5, Color { r: 120, g: 220, b: 90, a: 70 });
    dh.draw_circle_v(Vector2::new(ex, ey), bs as f32 * 0.3, ENEMIGO);

    let centro = Vector2::new(px, py);
    dh.draw_circle_v(centro, bs as f32 * 0.42, Color { r: 255, g: 110, b: 199, a: 60 });
    dh.draw_circle_v(centro, bs as f32 * 0.26, JUGADOR);
    dh.draw_line_ex(
        centro,
        Vector2::new(
            px + est.a.cos() * bs as f32 * 0.9,
            py + est.a.sin() * bs as f32 * 0.9,
        ),
        2.0,
        META,
    );
}

pub fn render_3d(
    dh: &mut RaylibDrawHandle<'_>,
    est: &Estado,
    texturas: &[Option<Texture2D>],
    usar_tex: bool,
) -> Vec<f32> {
    let hh = VIEW_H as f32 / 2.0;
    dh.draw_rectangle(0, 0, ANCHO, VIEW_H / 2, CIELO);
    dh.draw_rectangle(0, VIEW_H / 2, ANCHO, VIEW_H / 2, PISO);

    let n = (ANCHO / ANCHO_ESTACA) as usize;
    let mut zbuffer = vec![MAX_DIST; n];

    for i in 0..n {
        let t = i as f32 / n as f32;
        let ang = est.a - FOV / 2.0 + FOV * t;
        let imp = lanzar(&est.grid, est.x, est.y, ang);

        let d = (imp.d * (ang - est.a).cos()).max(0.05);
        zbuffer[i] = d;

        let stake_height = hh / d;
        let stake_top = hh - stake_height / 2.0;
        let stake_bottom = hh + stake_height / 2.0;

        let mut f = (1.6 / (1.0 + d * 0.55)).clamp(0.18, 1.0);
        if imp.ch == '-' {
            f *= 0.75;
        }
        let x = i as i32 * ANCHO_ESTACA;

        let idx = indice_textura(imp.ch);
        let tex_opt = texturas.get(idx).and_then(|t| t.as_ref());

        match (tex_opt, usar_tex) {
            (Some(tex), true) => {
                let tw = tex.width as f32;
                let th = tex.height as f32;
                let sx = (imp.tx * tw).clamp(0.0, tw - 1.0);
                let v = (255.0 * f) as u8;
                dh.draw_texture_pro(
                    tex,
                    Rectangle::new(sx, 0.0, 1.0, th),
                    Rectangle::new(x as f32, stake_top, ANCHO_ESTACA as f32, stake_height),
                    Vector2::zero(),
                    0.0,
                    Color { r: v, g: v, b: v, a: 255 },
                );
            }
            _ => {
                let base = match imp.ch {
                    '|' => Color { r: 178, g: 102, b: 235, a: 255 },
                    '-' => Color { r: 132, g: 62, b: 190, a: 255 },
                    'W' => PARED2,
                    'X' => PARED3,
                    'B' => META,
                    _ => PARED,
                };
                let top = stake_top.max(0.0) as i32;
                let bottom = stake_bottom.min(VIEW_H as f32) as i32;
                dh.draw_rectangle(x, top, ANCHO_ESTACA, (bottom - top).max(1), sombrear(base, f));
            }
        }
    }

    zbuffer
}


pub fn render_sprite(dh: &mut RaylibDrawHandle<'_>, est: &Estado, tex: &Texture2D, zbuffer: &[f32]) {
    let dx = est.ex - est.x;
    let dy = est.ey - est.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.05 {
        return;
    }

    let mut rel = dy.atan2(dx) - est.a;
    let dos_pi = std::f32::consts::PI * 2.0;
    while rel > std::f32::consts::PI {
        rel -= dos_pi;
    }
    while rel < -std::f32::consts::PI {
        rel += dos_pi;
    }
    if rel.abs() > FOV {
        return;
    }

    let hh = VIEW_H as f32 / 2.0;
    let tam = (VIEW_H as f32 / dist).min(VIEW_H as f32 * 3.0);
    let cx = ANCHO as f32 / 2.0 + (rel / (FOV / 2.0)) * (ANCHO as f32 / 2.0);
    let izq = cx - tam / 2.0;
    let top = hh - tam / 2.0;

    let dist_z = dist * rel.cos();

    let f = (1.8 / (1.0 + dist * 0.35)).clamp(0.35, 1.0);
    let tinte = Color {
        r: (255.0 * f) as u8,
        g: (255.0 * f) as u8,
        b: (255.0 * f) as u8,
        a: 255,
    };

    let tw = tex.width as f32;
    let th = tex.height as f32;
    let paso = ANCHO_ESTACA as f32;
    let sw = tw * (paso / tam);

    let mut s = 0.0f32;
    while s < tam {
        let x = izq + s;
        if x >= 0.0 && x < ANCHO as f32 {
            let col = (x as i32 / ANCHO_ESTACA) as usize;
            if col < zbuffer.len() && zbuffer[col] > dist_z {
                dh.draw_texture_pro(
                    tex,
                    Rectangle::new((s / tam) * tw, 0.0, sw, th),
                    Rectangle::new(x, top, paso, tam),
                    Vector2::zero(),
                    0.0,
                    tinte,
                );
            }
        }
        s += paso;
    }
}

pub fn render_minimapa(dh: &mut RaylibDrawHandle<'_>, est: &Estado) {
    let bs = (170 / est.cols()).clamp(2, 8);
    let (ox, oy) = (12, 12);
    let w = est.cols() * bs;
    let h = est.filas() * bs;
    

    dh.draw_rectangle(ox - 6, oy - 6, w + 12, h + 12, Color { r: 12, g: 5, b: 24, a: 200 });

    for (r, fila) in est.grid.iter().enumerate() {
        for (c, ch) in fila.iter().enumerate() {
            let col = color_de(*ch);
            dh.draw_rectangle(ox + c as i32 * bs, oy + r as i32 * bs, bs, bs, col);
        }
    }

    let ex = ox as f32 + est.ex * bs as f32;
    let ey = oy as f32 + est.ey * bs as f32;
    dh.draw_circle_v(Vector2::new(ex, ey), (bs as f32 * 0.5).max(2.0), ENEMIGO);

    let px = ox as f32 + est.x * bs as f32;
    let py = oy as f32 + est.y * bs as f32;
    dh.draw_line_ex(
        Vector2::new(px, py),
        Vector2::new(px + est.a.cos() * bs as f32 * 2.5, py + est.a.sin() * bs as f32 * 2.5),
        1.5,
        META,
    );
    dh.draw_circle_v(Vector2::new(px, py), (bs as f32 * 0.45).max(2.0), JUGADOR);
}
