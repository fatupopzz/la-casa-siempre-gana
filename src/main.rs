use raylib::prelude::*;
mod mapa;
mod raycast;
mod estado;
mod render;
use estado::Estado;
use render::{render_2d, render_3d, render_sprite, render_minimapa};


// -------------------------------------------------- ventana
pub const ANCHO: i32 = 960;
pub const ALTO: i32 = 640;
pub const HUD_H: i32 = 40;
pub const VIEW_H: i32 = ALTO - HUD_H;

// -------------------------------------------------- paleta
pub const BG: Color = Color { r: 26, g: 11, b: 46, a: 255 };
pub const PARED: Color = Color { r: 157, g: 78, b: 221, a: 255 };
pub const PARED_BORDE: Color = Color { r: 199, g: 125, b: 255, a: 255 };
pub const CAMINO: Color = Color { r: 42, g: 27, b: 61, a: 255 };
pub const INICIO: Color = Color { r: 116, g: 240, b: 200, a: 255 };
pub const META: Color = Color { r: 255, g: 110, b: 199, a: 255 };
pub const JUGADOR: Color = Color { r: 255, g: 224, b: 247, a: 255 };
pub const TEXTO: Color = Color { r: 224, g: 204, b: 255, a: 255 };
pub const RAYO: Color = Color { r: 255, g: 235, b: 250, a: 110 };
pub const RAYO_BORDE: Color = Color { r: 255, g: 110, b: 199, a: 220 };
pub const CIELO: Color = Color { r: 22, g: 9, b: 40, a: 255 };
pub const PISO: Color = Color { r: 38, g: 22, b: 58, a: 255 };
pub const ENEMIGO: Color = Color { r: 120, g: 220, b: 90, a: 255 };


// -------------------------------------------------- movimiento
const VEL: f32 = 3.2;
const VEL_GIRO: f32 = 2.4;
const SENS_MOUSE: f32 = 0.003;

// -------------------------------------------------- perseguidor
const DIST_ALERTA: f32 = 4.0; // a partir de aqui la pantalla se pone fea

// ==================================================== main
fn main() {
    let path = "maze.txt";
    let mut est = Estado::nuevo(path);

    let (mut rl, thread) = raylib::init()
        .size(ANCHO, ALTO)
        .title("Maze Runner - raycaster")
        .build();
    rl.set_target_fps(60);
    rl.disable_cursor();

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
