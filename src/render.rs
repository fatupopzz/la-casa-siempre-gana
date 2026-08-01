// render.rs — dibujo del mundo, sprites y minimapa

use raylib::prelude::*;
use crate::estado::Estado;
use crate::raycast::{lanzar, MAX_DIST};
use crate::mapa::es_pared;
use crate::{
    ANCHO, VIEW_H,
    PARED, PARED2, PARED3, PARED_BORDE, CAMINO, INICIO, META, JUGADOR,
    RAYO, RAYO_BORDE, ENEMIGO,
};

const FOV: f32 = std::f32::consts::PI / 3.0;
const ANCHO_ESTACA: i32 = 2;
const NUM_RAYOS_2D: usize = 80;

// --- niebla de tunel ---
const FOG: Color = Color { r: 6, g: 4, b: 4, a: 255 };
const FOG_DENSITY: f32 = 0.22;
const FOG_EDGE: f32 = 1.6;
const STEP_PISO: i32 = 2; // alto en px de cada tira del piso

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

/// col_t: 0.0 = centro de pantalla, 1.0 = borde
fn niebla(c: Color, d: f32, col_t: f32) -> Color {
    let dist_f = (-FOG_DENSITY * d).exp();
    let edge_f = 1.0 - col_t.powf(FOG_EDGE);
    let f = (dist_f * edge_f).clamp(0.0, 1.0);
    Color {
        r: (FOG.r as f32 + (c.r as f32 - FOG.r as f32) * f) as u8,
        g: (FOG.g as f32 + (c.g as f32 - FOG.g as f32) * f) as u8,
        b: (FOG.b as f32 + (c.b as f32 - FOG.b as f32) * f) as u8,
        a: 255,
    }
}

/// factor de niebla de tunel (0.0 = fog total, 1.0 = visible)
fn fog_factor(d: f32, col_t: f32) -> f32 {
    ((-FOG_DENSITY * d).exp() * (1.0 - col_t.powf(FOG_EDGE))).clamp(0.0, 1.0)
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
    tex_piso: Option<&Texture2D>,
    tex_techo: Option<&Texture2D>,
    usar_tex: bool,
) -> Vec<f32> {
    let hh = VIEW_H as f32 / 2.0;

    // fondo fog — el piso texturizado se pinta encima
    dh.draw_rectangle(0, 0, ANCHO, VIEW_H, FOG);

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

        let col_t = ((t - 0.5).abs() * 2.0).clamp(0.0, 1.0); // 0 centro, 1 borde
        let f_face: f32 = if imp.ch == '-' { 0.75 } else { 1.0 };
        let x = i as i32 * ANCHO_ESTACA;

        // ---- piso texturizado (floorcasting) ----
        if let Some(piso) = tex_piso {
            let pw = piso.width as f32;
            let ph = piso.height as f32;
            let cos_rel = (ang - est.a).cos().abs().max(0.001);
            let floor_start = (stake_bottom.floor() as i32).max(hh as i32 + 1);

            let mut y = floor_start;
            while y < VIEW_H {
                let p = y as f32 - hh;
                let row_dist = hh / p.max(0.5); // distancia perpendicular al piso

                let ff = fog_factor(row_dist, col_t);
                if ff < 0.03 { break; } // el resto es puro fog

                let ray_dist = row_dist / cos_rel;
                let wx = est.x + ang.cos() * ray_dist;
                let wy = est.y + ang.sin() * ray_dist;

                let sx = (wx.rem_euclid(1.0) * pw).clamp(0.0, pw - 1.0);
                let sy = (wy.rem_euclid(1.0) * ph).clamp(0.0, ph - 1.0);
                let v = (255.0 * ff) as u8;

                dh.draw_texture_pro(
                    piso,
                    Rectangle::new(sx, sy, 1.0, 1.0),
                    Rectangle::new(x as f32, y as f32, ANCHO_ESTACA as f32, STEP_PISO as f32),
                    Vector2::zero(),
                    0.0,
                    Color { r: v, g: v, b: v, a: v },
                );

                y += STEP_PISO;
            }
        }

        // ---- techo texturizado (ceilingcasting) ----
        if let Some(techo) = tex_techo {
            let cw = techo.width as f32;
            let ch = techo.height as f32;
            let cos_rel = (ang - est.a).cos().abs().max(0.001);
            let ceil_end = (stake_top.ceil() as i32).min(hh as i32);

            let mut y = ceil_end - STEP_PISO;
            while y >= 0 {
                let p = hh - y as f32;
                let row_dist = hh / p.max(0.5);

                let ff = fog_factor(row_dist, col_t);
                if ff < 0.03 { break; }

                let ray_dist = row_dist / cos_rel;
                let wx = est.x + ang.cos() * ray_dist;
                let wy = est.y + ang.sin() * ray_dist;

                let sx = (wx.rem_euclid(1.0) * cw).clamp(0.0, cw - 1.0);
                let sy = (wy.rem_euclid(1.0) * ch).clamp(0.0, ch - 1.0);
                let v = (255.0 * ff) as u8;

                dh.draw_texture_pro(
                    techo,
                    Rectangle::new(sx, sy, 1.0, 1.0),
                    Rectangle::new(x as f32, y as f32, ANCHO_ESTACA as f32, STEP_PISO as f32),
                    Vector2::zero(),
                    0.0,
                    Color { r: v, g: v, b: v, a: v },
                );

                y -= STEP_PISO;
            }
        }

        // ---- estaca (pared) ----
        let idx = indice_textura(imp.ch);
        let tex_opt = texturas.get(idx).and_then(|t| t.as_ref());

        match (tex_opt, usar_tex) {
            (Some(tex), true) => {
                let tw = tex.width as f32;
                let th = tex.height as f32;
                let sx = (imp.tx * tw).clamp(0.0, tw - 1.0);
                let v = (255.0 * f_face) as u8;
                dh.draw_texture_pro(
                    tex,
                    Rectangle::new(sx, 0.0, 1.0, th),
                    Rectangle::new(x as f32, stake_top, ANCHO_ESTACA as f32, stake_height),
                    Vector2::zero(),
                    0.0,
                    Color { r: v, g: v, b: v, a: 255 },
                );
                // overlay de niebla
                let fog_a = ((1.0 - fog_factor(d, col_t)) * 255.0) as u8;
                let top_i = stake_top.max(0.0) as i32;
                let h_i = (stake_height.min(VIEW_H as f32 - stake_top.max(0.0))).max(1.0) as i32;
                dh.draw_rectangle(
                    x, top_i, ANCHO_ESTACA, h_i,
                    Color { r: FOG.r, g: FOG.g, b: FOG.b, a: fog_a },
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
                let base_dim = sombrear(base, f_face);
                dh.draw_rectangle(
                    x, top, ANCHO_ESTACA, (bottom - top).max(1),
                    niebla(base_dim, d, col_t),
                );
            }
        }
    }

    zbuffer
}

pub fn render_sprite(
    dh: &mut RaylibDrawHandle<'_>,
    est: &Estado,
    tex: &Texture2D,
    zbuffer: &[f32],
) {
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

    let spr_col_t = ((cx / ANCHO as f32) - 0.5).abs() * 2.0;
    let ff = fog_factor(dist, spr_col_t);
    let fog_a = ((1.0 - ff) * 255.0) as u8;

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
                    Color::WHITE,
                );
                dh.draw_rectangle(
                    x as i32, top as i32, paso as i32, tam as i32,
                    Color { r: FOG.r, g: FOG.g, b: FOG.b, a: fog_a },
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

/// vineta: oscurece arriba y abajo
pub fn render_vigneta(dh: &mut RaylibDrawHandle<'_>) {
    let bandas = 20i32;
    let alto_banda = VIEW_H / bandas;
    for i in 0..bandas / 3 {
        let t = 1.0 - (i as f32 / (bandas as f32 / 3.0));
        let a = (t * t * 180.0) as u8;
        dh.draw_rectangle(0, i * alto_banda, ANCHO, alto_banda + 1,
            Color { r: 0, g: 0, b: 0, a });
    }
    for i in 0..bandas / 3 {
        let t = i as f32 / (bandas as f32 / 3.0);
        let a = (t * t * 180.0) as u8;
        dh.draw_rectangle(
            0, VIEW_H - (bandas / 3 - i) * alto_banda,
            ANCHO, alto_banda + 1,
            Color { r: 0, g: 0, b: 0, a },
        );
    }
}
