// Maze Runner - raycaster con perseguidor
//
//   maze.txt:  + - |  pared      A inicio      B meta      espacio = camino
//
//   Modo 2D  -> el laberinto de arriba, con los rayos del FOV como lineas.
//   Modo 3D  -> primera persona: una estaca por rayo, altura = hh / distancia.
//   El perseguidor te busca con BFS y se dibuja como sprite con z-buffer,
//   asi que las paredes lo tapan de verdad.
//
// uso: cargo run --release
//      cargo run --bin gen -- 15 11 2     <- regenera maze.txt

use raylib::prelude::*;
mod mapa;
mod raycast;
mod estado;
use raycast::{lanzar, Impacto, MAX_DIST};
use mapa::{cargar, buscar, es_pared, char_en, libre, campo_desde, RADIO};
use estado::Estado;


// -------------------------------------------------- ventana
const ANCHO: i32 = 960;
const ALTO: i32 = 640;
const HUD_H: i32 = 40;
const VIEW_H: i32 = ALTO - HUD_H;

// -------------------------------------------------- paleta
const BG: Color = Color { r: 26, g: 11, b: 46, a: 255 };
const PARED: Color = Color { r: 157, g: 78, b: 221, a: 255 };
const PARED_BORDE: Color = Color { r: 199, g: 125, b: 255, a: 255 };
const CAMINO: Color = Color { r: 42, g: 27, b: 61, a: 255 };
const INICIO: Color = Color { r: 116, g: 240, b: 200, a: 255 };
const META: Color = Color { r: 255, g: 110, b: 199, a: 255 };
const JUGADOR: Color = Color { r: 255, g: 224, b: 247, a: 255 };
const TEXTO: Color = Color { r: 224, g: 204, b: 255, a: 255 };
const RAYO: Color = Color { r: 255, g: 235, b: 250, a: 110 };
const RAYO_BORDE: Color = Color { r: 255, g: 110, b: 199, a: 220 };
const CIELO: Color = Color { r: 22, g: 9, b: 40, a: 255 };
const PISO: Color = Color { r: 38, g: 22, b: 58, a: 255 };
const ENEMIGO: Color = Color { r: 120, g: 220, b: 90, a: 255 };

// -------------------------------------------------- raycasting
const FOV: f32 = std::f32::consts::PI / 3.0;
const ANCHO_ESTACA: i32 = 2;
const NUM_RAYOS_2D: usize = 80;

// -------------------------------------------------- movimiento
const VEL: f32 = 3.2;
const VEL_GIRO: f32 = 2.4;

// -------------------------------------------------- perseguidor
const DIST_ALERTA: f32 = 4.0; // a partir de aqui la pantalla se pone fea

fn color_de(ch: char) -> Color {
    match ch {
        '+' | '-' | '|' => PARED,
        'A' => INICIO,
        'B' => META,
        _ => CAMINO,
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


// ==================================================== vista 2D
fn render_2d(dh: &mut RaylibDrawHandle<'_>, est: &Estado) {
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

    // el perseguidor
    let ex = ox as f32 + est.ex * bs as f32;
    let ey = oy as f32 + est.ey * bs as f32;
    dh.draw_circle_v(Vector2::new(ex, ey), bs as f32 * 0.5, Color { r: 120, g: 220, b: 90, a: 70 });
    dh.draw_circle_v(Vector2::new(ex, ey), bs as f32 * 0.3, ENEMIGO);

    // el jugador
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

// ==================================================== vista 3D
/// Dibuja las estacas y devuelve el z-buffer: la distancia de cada columna.
fn render_3d(
    dh: &mut RaylibDrawHandle<'_>,
    est: &Estado,
    juanjo: Option<&Texture2D>,
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

        match (juanjo, usar_tex) {
            (Some(tex), true) => {
                let tw = tex.width as f32;
                let th = tex.height as f32;
                let sx = (imp.tx * tw).clamp(0.0, tw - 1.0);
                dh.draw_texture_pro(
                    tex,
                    Rectangle::new(sx, 0.0, 1.0, th),
                    Rectangle::new(x as f32, stake_top, ANCHO_ESTACA as f32, stake_height),
                    Vector2::zero(),
                    0.0,
                    Color {
                        r: (250.0 * f) as u8,
                        g: (228.0 * f) as u8,
                        b: (255.0 * f) as u8,
                        a: 255,
                    },
                );
            }
            _ => {
                let base = match imp.ch {
                    '|' => Color { r: 178, g: 102, b: 235, a: 255 },
                    '-' => Color { r: 132, g: 62, b: 190, a: 255 },
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

// ==================================================== sprite del perseguidor
/// Billboard: siempre de frente a la camara, tapado por las paredes gracias al z-buffer.
fn render_sprite(dh: &mut RaylibDrawHandle<'_>, est: &Estado, tex: &Texture2D, zbuffer: &[f32]) {
    let dx = est.ex - est.x;
    let dy = est.ey - est.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.05 {
        return;
    }

    // angulo del sprite respecto a hacia donde estamos viendo
    let mut rel = dy.atan2(dx) - est.a;
    let dos_pi = std::f32::consts::PI * 2.0;
    while rel > std::f32::consts::PI {
        rel -= dos_pi;
    }
    while rel < -std::f32::consts::PI {
        rel += dos_pi;
    }
    // si esta muy afuera del cono, ni lo intentamos
    if rel.abs() > FOV {
        return;
    }

    let hh = VIEW_H as f32 / 2.0;
    // mismo criterio de tamaño que las estacas: inversamente proporcional a la distancia
    let tam = (VIEW_H as f32 / dist).min(VIEW_H as f32 * 3.0);
    // el barrido de angulos es lineal, asi que la x en pantalla tambien
    let cx = ANCHO as f32 / 2.0 + (rel / (FOV / 2.0)) * (ANCHO as f32 / 2.0);
    let izq = cx - tam / 2.0;
    let top = hh - tam / 2.0;

    // el z-buffer guarda distancias perpendiculares, asi que hay que comparar
    // contra la perpendicular del sprite, no la radial
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
    let sw = tw * (paso / tam); // cuanta textura cabe en una tira de pantalla

    let mut s = 0.0f32;
    while s < tam {
        let x = izq + s;
        if x >= 0.0 && x < ANCHO as f32 {
            let col = (x as i32 / ANCHO_ESTACA) as usize;
            // aqui esta la gracia: solo se dibuja si no hay pared mas cerca
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

// ==================================================== minimapa
fn render_minimapa(dh: &mut RaylibDrawHandle<'_>, est: &Estado) {
    let bs = (170 / est.cols()).clamp(2, 8);
    let (ox, oy) = (12, 12);
    let w = est.cols() * bs;
    let h = est.filas() * bs;

    dh.draw_rectangle(ox - 6, oy - 6, w + 12, h + 12, Color { r: 12, g: 5, b: 24, a: 200 });

    for (r, fila) in est.grid.iter().enumerate() {
        for (c, ch) in fila.iter().enumerate() {
            let col = match ch {
                '+' | '-' | '|' => PARED,
                'B' => META,
                _ => CAMINO,
            };
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

// ==================================================== main
fn main() {
    let path = "maze.txt";
    let mut est = Estado::nuevo(path);

    let (mut rl, thread) = raylib::init()
        .size(ANCHO, ALTO)
        .title("Maze Runner - raycaster")
        .build();
    rl.set_target_fps(60);

    let juanjo = rl.load_texture(&thread, "assets/juanjo.png").ok();
    if juanjo.is_none() {
        println!("aviso: no encontre assets/juanjo.png, van paredes de color plano");
    }
    let perseguidor = rl.load_texture(&thread, "assets/perseguidor.png").ok();
    if perseguidor.is_none() {
        println!("aviso: no encontre assets/perseguidor.png, el enemigo va invisible en 3D");
    }
    let mut usar_tex = true;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let vivo = !est.gano && !est.atrapado;

        // ---------------- input
        if vivo {
            if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
                est.a -= VEL_GIRO * dt;
            }
            if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
                est.a += VEL_GIRO * dt;
            }

            let paso = VEL * dt;
            if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
                est.avanzar(est.a.cos() * paso, est.a.sin() * paso);
            }
            if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
                est.avanzar(-est.a.cos() * paso, -est.a.sin() * paso);
            }
            let lado = est.a + std::f32::consts::FRAC_PI_2;
            if rl.is_key_down(KeyboardKey::KEY_Q) {
                est.avanzar(-lado.cos() * paso, -lado.sin() * paso);
            }
            if rl.is_key_down(KeyboardKey::KEY_E) {
                est.avanzar(lado.cos() * paso, lado.sin() * paso);
            }

            est.perseguir(dt);
        }

        if rl.is_key_pressed(KeyboardKey::KEY_M) {
            est.modo3d = !est.modo3d;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            usar_tex = !usar_tex;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            est = Estado::nuevo(path);
        }

        let fps = rl.get_fps();
        let dist_e = est.dist_enemigo();

        // ---------------- dibujo
        let mut dh = rl.begin_drawing(&thread);
        dh.clear_background(BG);

        if est.modo3d {
            let zbuffer = render_3d(&mut dh, &est, juanjo.as_ref(), usar_tex);
            if let Some(tex) = &perseguidor {
                render_sprite(&mut dh, &est, tex, &zbuffer);
            }
            render_minimapa(&mut dh, &est);
        } else {
            render_2d(&mut dh, &est);
        }

        // se te acerca: la pantalla se pone verde
        if vivo && dist_e < DIST_ALERTA {
            let intensidad = (1.0 - dist_e / DIST_ALERTA).clamp(0.0, 1.0);
            dh.draw_rectangle(
                0,
                0,
                ANCHO,
                VIEW_H,
                Color { r: 60, g: 140, b: 40, a: (intensidad * 70.0) as u8 },
            );
        }

        // ---------------- hud
        dh.draw_rectangle(0, VIEW_H, ANCHO, HUD_H, Color { r: 16, g: 7, b: 30, a: 255 });
        let modo = if est.modo3d { "3D" } else { "2D" };
        dh.draw_text(
            &format!("[{}]  M vista  T textura  WASD mover  Q/E de lado  R reinicia", modo),
            12,
            VIEW_H + 12,
            16,
            TEXTO,
        );
        dh.draw_text(
            &format!("el sujeto a {:.1}   {} fps", dist_e, fps),
            ANCHO - 210,
            VIEW_H + 12,
            16,
            if dist_e < DIST_ALERTA { ENEMIGO } else { Color { r: 140, g: 110, b: 180, a: 255 } },
        );

        // ---------------- finales
        if est.atrapado {
            dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 20, g: 45, b: 15, a: 215 });
            if let Some(tex) = &perseguidor {
                let sz = 260.0;
                dh.draw_texture_pro(
                    tex,
                    Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                    Rectangle::new(ANCHO as f32 / 2.0 - sz / 2.0, VIEW_H as f32 / 2.0 - sz + 40.0, sz, sz),
                    Vector2::zero(),
                    0.0,
                    Color::WHITE,
                );
            }
            dh.draw_text("te alcanzo", ANCHO / 2 - 95, VIEW_H / 2 + 60, 34, ENEMIGO);
            dh.draw_text("R para reiniciar", ANCHO / 2 - 68, VIEW_H / 2 + 106, 18, TEXTO);
        } else if est.gano {
            dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 26, g: 11, b: 46, a: 215 });
            if let Some(tex) = &juanjo {
                let sz = 190.0;
                dh.draw_texture_pro(
                    tex,
                    Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                    Rectangle::new(ANCHO as f32 / 2.0 - sz / 2.0, VIEW_H as f32 / 2.0 - sz, sz, sz),
                    Vector2::zero(),
                    0.0,
                    Color::WHITE,
                );
            }
            dh.draw_text("escapaste", ANCHO / 2 - 88, VIEW_H / 2 + 20, 34, META);
            dh.draw_text("R para reiniciar", ANCHO / 2 - 68, VIEW_H / 2 + 66, 18, TEXTO);
        }
    }
}
