use raylib::prelude::*;
mod mapa;
mod raycast;
mod estado;
mod render;
mod juego;
use estado::Estado;
use render::{render_2d, render_3d, render_sprite, render_minimapa};
use juego::Escena;

// -------------------------------------------------- ventana
pub const ANCHO: i32 = 960;
pub const ALTO: i32 = 640;
pub const HUD_H: i32 = 40;
pub const VIEW_H: i32 = ALTO - HUD_H;


// -------------------------------------------------- paleta
pub const BG: Color = Color { r: 18, g: 8, b: 8, a: 255 };
pub const PARED: Color = Color { r: 74, g: 64, b: 57, a: 255 };
pub const PARED_BORDE: Color = Color { r: 90, g: 74, b: 64, a: 255 };
pub const PARED2: Color = Color { r: 72, g: 85, b: 80, a: 255 };
pub const PARED3: Color = Color { r: 90, g: 56, b: 34, a: 255 };
pub const CAMINO: Color = Color { r: 30, g: 20, b: 18, a: 255 };
pub const INICIO: Color = Color { r: 136, g: 68, b: 34, a: 255 };
pub const META: Color = Color { r: 204, g: 34, b: 34, a: 255 };
pub const JUGADOR: Color = Color { r: 255, g: 207, b: 188, a: 255 };
pub const TEXTO: Color = Color { r: 196, g: 168, b: 130, a: 255 };
pub const RAYO: Color = Color { r: 255, g: 107, b: 80, a: 64 };
pub const RAYO_BORDE: Color = Color { r: 204, g: 34, b: 34, a: 204 };
pub const CIELO: Color = Color { r: 14, g: 6, b: 6, a: 255 };
pub const PISO: Color = Color { r: 58, g: 31, b: 34, a: 255 };
pub const ENEMIGO: Color = Color { r: 120, g: 220, b: 90, a: 255 };

// -------------------------------------------------- movimiento
const VEL: f32 = 3.2;
const VEL_GIRO: f32 = 2.4;
const SENS_MOUSE: f32 = 0.003;

// -------------------------------------------------- perseguidor
const DIST_ALERTA: f32 = 4.0;

// -------------------------------------------------- pisos
const MAPAS: [&str; 3] = ["mapas/piso1.txt", "mapas/piso2.txt", "mapas/piso3.txt"];

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(ANCHO, ALTO)
        .title("La casa siempre gana")
        .build();
    rl.set_target_fps(60);
    rl.disable_cursor();

    let juanjo = rl.load_texture(&thread, "assets/juanjo.png").ok();
    if juanjo.is_none() {
        println!("aviso: no encontre assets/juanjo.png, van paredes de color plano");
    }
    let perseguidor_tex = rl.load_texture(&thread, "assets/perseguidor.png").ok();
    if perseguidor_tex.is_none() {
        println!("aviso: no encontre assets/perseguidor.png, el enemigo va invisible en 3D");
    }

   let mut texturas: Vec<Option<Texture2D>> = Vec::new();
    for p in ["assets/texturas/concreto.png", "assets/texturas/azulejo.png", "assets/texturas/metal.png"] {
        let t = rl.load_texture(&thread, p).ok();
        if t.is_none() {
            println!("aviso: no encontre {}", p);
        }
        texturas.push(t);
    }

    let tex_piso = rl.load_texture(&thread, "assets/texturas/alfombra.png").ok();
    if tex_piso.is_none() {
        println!("aviso: no encontre assets/texturas/alfombra.png");
    }

    let tex_techo = rl.load_texture(&thread, "assets/texturas/concreto.png").ok();

    let mut escena = Escena::Bienvenida;
    let mut est = Estado::nuevo(MAPAS[0]);
    let mut usar_tex = true;
    let mut piso: usize = 0;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();

        match escena {
            // ============================================ BIENVENIDA
            Escena::Bienvenida => {
                if rl.is_key_pressed(KeyboardKey::KEY_ONE) { piso = 0; }
                if rl.is_key_pressed(KeyboardKey::KEY_TWO) { piso = 1; }
                if rl.is_key_pressed(KeyboardKey::KEY_THREE) { piso = 2; }
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    est = Estado::nuevo(MAPAS[piso]);
                    est.modo3d = true;
                    escena = Escena::Jugando;
                }

                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(BG);
                dh.draw_text("LA CASA SIEMPRE GANA", ANCHO / 2 - 200, 180, 36, META);
                dh.draw_text(
                    &format!("> Piso {} <", piso + 1),
                    ANCHO / 2 - 60, 260, 24, TEXTO,
                );
                dh.draw_text("1  2  3  para elegir piso", ANCHO / 2 - 110, 320, 18, TEXTO);
                dh.draw_text("ENTER para empezar", ANCHO / 2 - 85, 360, 18, META);
            }

            // ============================================ JUGANDO
            Escena::Jugando => {
                // input
                if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
                    est.a -= VEL_GIRO * dt;
                }
                if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
                    est.a += VEL_GIRO * dt;
                }
                let mouse_dx = rl.get_mouse_delta().x;
                est.a += mouse_dx * SENS_MOUSE;

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

                if rl.is_key_pressed(KeyboardKey::KEY_M) {
                    est.modo3d = !est.modo3d;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_T) {
                    usar_tex = !usar_tex;
                }

                // transiciones
                if est.gano {
                    escena = Escena::Victoria;
                }
                if est.atrapado {
                    escena = Escena::Derrota;
                }

                // dibujo
                let fps = rl.get_fps();
                let dist_e = est.dist_enemigo();
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(BG);

                if est.modo3d {
                  let zbuffer = render_3d(&mut dh, &est, &texturas, tex_piso.as_ref(), tex_techo.as_ref(), usar_tex);
                    if let Some(tex) = &perseguidor_tex {
                        render_sprite(&mut dh, &est, tex, &zbuffer);
                    }
                    render_minimapa(&mut dh, &est);
                } else {
                    render_2d(&mut dh, &est);
                }

                if dist_e < DIST_ALERTA {
                    let intensidad = (1.0 - dist_e / DIST_ALERTA).clamp(0.0, 1.0);
                    dh.draw_rectangle(0, 0, ANCHO, VIEW_H,
                        Color { r: 60, g: 140, b: 40, a: (intensidad * 70.0) as u8 });
                }

                dh.draw_rectangle(0, VIEW_H, ANCHO, HUD_H, Color { r: 16, g: 7, b: 30, a: 255 });
                let modo = if est.modo3d { "3D" } else { "2D" };
                dh.draw_text(
                    &format!("[{}]  M vista  T textura  WASD mover  Q/E de lado", modo),
                    12, VIEW_H + 12, 16, TEXTO,
                );
                dh.draw_text(
                    &format!("el sujeto a {:.1}   {} fps", dist_e, fps),
                    ANCHO - 210, VIEW_H + 12, 16,
                    if dist_e < DIST_ALERTA { ENEMIGO }
                    else { Color { r: 140, g: 110, b: 180, a: 255 } },
                );
            }

            // ============================================ VICTORIA
            Escena::Victoria => {
                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    escena = Escena::Bienvenida;
                }

                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(BG);
                dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 26, g: 11, b: 46, a: 215 });
                if let Some(tex) = &juanjo {
                    let sz = 190.0;
                    dh.draw_texture_pro(
                        tex,
                        Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                        Rectangle::new(ANCHO as f32 / 2.0 - sz / 2.0, VIEW_H as f32 / 2.0 - sz, sz, sz),
                        Vector2::zero(), 0.0, Color::WHITE,
                    );
                }
                dh.draw_text("escapaste", ANCHO / 2 - 88, VIEW_H / 2 + 20, 34, META);
                dh.draw_text("R para volver al menu", ANCHO / 2 - 95, VIEW_H / 2 + 66, 18, TEXTO);
            }

            // ============================================ DERROTA
            Escena::Derrota => {
                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    escena = Escena::Bienvenida;
                }

                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(BG);
                dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 20, g: 45, b: 15, a: 215 });
                if let Some(tex) = &perseguidor_tex {
                    let sz = 260.0;
                    dh.draw_texture_pro(
                        tex,
                        Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                        Rectangle::new(ANCHO as f32 / 2.0 - sz / 2.0, VIEW_H as f32 / 2.0 - sz + 40.0, sz, sz),
                        Vector2::zero(), 0.0, Color::WHITE,
                    );
                }
                dh.draw_text("te alcanzo", ANCHO / 2 - 95, VIEW_H / 2 + 60, 34, ENEMIGO);
                dh.draw_text("R para volver al menu", ANCHO / 2 - 95, VIEW_H / 2 + 106, 18, TEXTO);
            }
        }
    }
}
