use rand::Rng;
use raylib::prelude::*;
mod mapa;
mod raycast;
mod estado;
mod render;
mod juego;
mod maquina;
use maquina::{AnimRodillos, Maquina, Simbolo};
use mapa::hay_adyacente;
use estado::Estado;
use render::{render_2d, render_3d, render_minimapa};
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

// -------------------------------------------------- pisos
/// Todo lo que hace que un piso se sienta distinto sin tocar los mapas: la
/// cuota que hay que pegarle a la maquina y que tan encima se te viene la
/// sombra. Es la unica fuente de verdad; no quedan consts globales de esto.
#[derive(Clone, Copy)]
struct ConfigPiso {
    nombre: &'static str,
    mapa: &'static str,
    cuota: i32,
    giros: i32,
    vel_enemigo: f32,
    dist_alerta: f32,
}

// Las tres velocidades estan arriba de VEL (3.2) a proposito: no se le escapa
// corriendo en ningun piso. Lo que cambia entre pisos es cuanto margen tenes
// para llegar a la salida, no si podes o no dejarla atras.
const PISOS: [ConfigPiso; 3] = [
    ConfigPiso {
        nombre: "PISO 1",
        mapa: "mapas/piso1.txt",
        cuota: 100,
        giros: 20,
        vel_enemigo: 3.8,
        dist_alerta: 6.0,
    },
    ConfigPiso {
        nombre: "PISO 2",
        mapa: "mapas/piso2.txt",
        cuota: 170,
        giros: 16,
        vel_enemigo: 4.1,
        dist_alerta: 5.0,
    },
    ConfigPiso {
        nombre: "PISO 3",
        mapa: "mapas/piso3.txt",
        cuota: 250,
        giros: 12,
        vel_enemigo: 4.4,
        dist_alerta: 4.0,
    },
];

/// arranca el laberinto de un piso: su mapa, la sombra suelta y a la velocidad
/// que le toca. Todo sale del ConfigPiso, no hay consts de por medio.
fn entrar_al_laberinto(cfg: &ConfigPiso) -> Estado {
    let mut est = Estado::nuevo(cfg.mapa, cfg.vel_enemigo);
    est.modo3d = true;
    est.persiguiendo = true;
    est
}

/// pasa al piso siguiente, o Victoria si era el ultimo
fn siguiente_ronda(
    piso: &mut usize,
    maq: &mut Maquina,
    anim: &mut AnimRodillos,
    en_laberinto: &mut bool,
) -> Escena {
    if *piso + 1 >= PISOS.len() {
        return Escena::Victoria;
    }
    *piso += 1;
    let cfg = PISOS[*piso];
    *maq = Maquina::nueva(cfg.cuota, cfg.giros);
    *anim = AnimRodillos::nueva();
    *en_laberinto = false;
    Escena::Maquina
}

// -------------------------------------------------- fundido entre escenas
const T_FUNDIDO: f32 = 0.25; // dura lo mismo cerrar que abrir

/// Fundido a negro para entrar y salir de la maquina: el corte de 3D a 2D a
/// pantalla completa se siente brusco sin esto. Primero cierra a negro, ahi
/// cambia la escena, y despues abre. El cambio no pasa hasta que la pantalla
/// esta negra del todo, asi no se ve el salto.
struct Fundido {
    t: f32,
    destino: Option<Escena>,
}

impl Fundido {
    fn nuevo() -> Self {
        Fundido { t: 0.0, destino: None }
    }

    /// pide el cambio de escena. Si ya hay uno en curso no lo pisa. El reloj
    /// NO se reinicia: si venia abriendo, cierra desde donde estaba el velo en
    /// vez de saltar a transparente y volver a oscurecer.
    fn ir_a(&mut self, e: Escena) {
        if self.destino.is_none() {
            self.destino = Some(e);
        }
    }

    /// hay un cambio en curso: mientras tanto la escena no toma input
    fn en_curso(&self) -> bool {
        self.destino.is_some()
    }

    /// avanza el reloj. Devuelve Some(escena) en el unico frame en que hay que
    /// cambiarla: justo cuando el velo llego a negro.
    fn actualizar(&mut self, dt: f32) -> Option<Escena> {
        if self.destino.is_some() {
            self.t += dt;
            if self.t >= T_FUNDIDO {
                self.t = T_FUNDIDO; // desde aca arranca la apertura
                return self.destino.take();
            }
        } else if self.t > 0.0 {
            self.t = (self.t - dt).max(0.0);
        }
        None
    }

    /// velo negro encima de todo. Va al final de cada escena que participa.
    fn velo(&self, dh: &mut RaylibDrawHandle<'_>) {
        let a = ((self.t / T_FUNDIDO).clamp(0.0, 1.0) * 255.0) as u8;
        if a > 0 {
            dh.draw_rectangle(0, 0, ANCHO, ALTO, Color { r: 0, g: 0, b: 0, a });
        }
    }
}

// -------------------------------------------------- neon de la maquina
const NEON: Color = Color { r: 255, g: 110, b: 199, a: 255 };
const CYAN: Color = Color { r: 125, g: 249, b: 255, a: 255 };
const DORADO: Color = Color { r: 255, g: 215, b: 0, a: 255 };
const RODILLO_BG: Color = Color { r: 26, g: 26, b: 26, a: 255 };

// -------------------------------------------------- marquesina de bienvenida
// Todo va como fraccion de ANCHO/ALTO: si cambia el tamano de ventana, el
// letrero se reacomoda solo. Los colores saturados (NEON = #FF6EC7,
// CYAN = #7DF9FF) son los mismos que ya usa la maquina.
const GRANATE: Color = Color { r: 90, g: 30, b: 34, a: 255 };        // #5A1E22
const GRANATE_OSCURO: Color = Color { r: 58, g: 18, b: 22, a: 255 }; // #3A1216
const RELLENO_MARQ: Color = Color { r: 8, g: 4, b: 6, a: 255 };      // casi negro

const MARQ_W: f32 = 0.70;      // ancho, fraccion de ANCHO
const MARQ_H: f32 = 0.445;     // alto, fraccion de ALTO
const MARQ_Y: f32 = 0.03;      // tope, fraccion de ALTO
const MARQ_BORDE: f32 = 0.011; // grosor del borde granate, fraccion de ANCHO

// marco art deco. El alto NO se escribe: sale del ancho / la relacion del png.
const RUTA_MARCO: &str = "assets/sprites/marquesina.png";
const MARCO_PX: (f32, f32) = (800.0, 490.0); // rect fuente completo
const MARCO_RATIO: f32 = 1.6327;             // 800 / 490
const MARCO_ANCHO: f32 = 0.62;               // fraccion de ANCHO
const MARCO_Y: f32 = 0.012;                  // tope, fraccion de ALTO
// hueco interno donde entra el logo, normalizado sobre el marco
const MARCO_HUECO: [f32; 4] = [0.0750, 0.2653, 0.8438, 0.5306]; // x, y, w, h
const MARGEN_LOGO: f32 = 0.92; // el logo ocupa el 92% del hueco

// Los remaches del marco vienen dibujados en la textura: no se dibuja nada
// encima. El rosa queda solo para el logo, todo lo demas va en laton.
const LATON: Color = Color { r: 150, g: 124, b: 66, a: 255 };
const LATON_TENUE: Color = Color { r: 108, g: 88, b: 48, a: 255 };

// logo: el tamano sale del hueco del marco, y el alto de la relacion del png
const RUTA_LOGO: &str = "assets/sprites/logo.png";
const LOGO_PX: (f32, f32) = (536.0, 200.0); // rect fuente completo
const LOGO_RATIO: f32 = 2.68;               // 536 / 200

// titulo con fuente: solo se usa como fallback si no esta el png del logo
const TAM_TITULO: f32 = 0.13;    // fraccion de ALTO
const TAM_SUBTITULO: f32 = 0.07;
const GAP_TITULO: f32 = 0.02;

// placas de piso: spritesheet de 800x480, celdas de 400x160 (relacion 2.5:1).
// columna 0 = apagada, columna 1 = encendida, fila = indice del piso.
const RUTA_PLACAS: &str = "assets/sprites/placas.png";
const PLACA_W: f32 = 400.0; // celda del spritesheet
const PLACA_H: f32 = 160.0;
const PLACA_RATIO: f32 = 2.5;   // 400 / 160, de aca sale el alto destino
// van en una linea horizontal, que es como se navega con A/D
const PLACA_ANCHO: f32 = 0.28;  // fraccion de ANCHO -> 269x108
const PLACAS_Y: f32 = 0.655;    // tope de la fila, fraccion de ALTO
const PLACA_BORDE: f32 = 0.0028;// grosor del borde, solo para el fallback
const TAM_PLACA: f32 = 0.042;   // tamano del nombre, solo para el fallback

const INFO_Y: f32 = 0.885;      // cuota/giros del piso elegido
const CONTROLES_Y: f32 = 0.936;
const TAM_CHICO: f32 = 0.034;   // fraccion de ALTO

// flicker del letrero: dos frecuencias, la segunda irregular. El piso no se
// negocia, abajo de eso el titulo se vuelve ilegible y parece bug, no atmosfera.
const FLICKER_MIN: f32 = 0.7;
const FLICKER_MAX: f32 = 1.05;

// fondo de alfombra de teatro. Va oscuro y desaturado a proposito: tiene que
// leerse como alfombra roja, no competirle al letrero. El degrade y el tejido
// en rombo son la diferencia entre "alfombra" y "un rojo plano".
const ALFOMBRA: Color = Color { r: 46, g: 12, b: 17, a: 255 };       // base, abajo
const ALFOMBRA_ALTO: Color = Color { r: 20, g: 6, b: 9, a: 255 };    // arriba, mas apagado
const ALFOMBRA_HILO: Color = Color { r: 122, g: 40, b: 48, a: 255 }; // el tejido
const ALFOMBRA_BANDAS: i32 = 32; // franjas del degrade vertical
const ROMBO_PASO: f32 = 0.05;    // separacion del tejido, fraccion de ANCHO
const ROMBO_GROSOR: f32 = 2.0;
// el tejido se dibuja con mas alpha que antes justamente porque el fondo va a
// ALFOMBRA_BRILLO: si no, al bajar el brillo la textura desaparecia
const ROMBO_ALPHA: u8 = 50;
// la alfombra va a un cuarto de brillo: es textura de fondo, no protagonista
const ALFOMBRA_BRILLO: f32 = 0.25;

// vineta y grano
const VINETA_BIENVENIDA: (f32, f32) = (0.15, 225.0); // (alcance, alpha maximo)
const VINETA_MAQUINA: (f32, f32) = (0.20, 200.0);
const GRANO_RECTS: usize = 1200; // rects sueltos, no un for pixel por pixel
const GRANO_PX: i32 = 2;
const GRANO_ALPHA: u8 = 25;

// -------------------------------------------------- cuarto de la maquina
// Fondo 1:1 con la ventana. Viene dibujado muy oscuro a proposito: no se le
// sube el brillo, el gabinete tiene que seguir siendo lo unico saturado.
const RUTA_FONDO_MAQUINA: &str = "assets/texturas/fondo_maquina.png";
const FONDO_MAQUINA_PX: (f32, f32) = (960.0, 640.0);
// el cuerpo de la camara de vigilancia viene en la textura; el ojo va en codigo
// para que pulse. Es un punto que notas de reojo, no una lampara.
const OJO_CAMARA: (f32, f32) = (0.8125, 0.0938); // normalizado
const OJO_RADIO: f32 = 3.0;
const OJO_NUCLEO: f32 = 1.5;
const OJO_ROJO: Color = Color { r: 200, g: 40, b: 40, a: 255 };
const OJO_BRILLO: Color = Color { r: 255, g: 140, b: 120, a: 255 };

// -------------------------------------------------- gabinete de la tragamonedas
// Todo lo tuneable del arte vive aca. Las coordenadas van normalizadas 0-1 sobre
// el lado del gabinete, asi que mover LADO_GABINETE reacomoda todo solo.
const RUTA_GABINETE: &str = "assets/sprites/gabinete.png";
const GABINETE_PX: f32 = 768.0;                // lado del png fuente
const LADO_GABINETE: f32 = ALTO as f32 * 0.78; // lado que ocupa en pantalla
// centros de las tres ventanas negras
const CENTROS_SLOT: [f32; 3] = [0.2812, 0.5000, 0.7188];
const CENTRO_Y_SLOT: f32 = 0.5195;
const ANCHO_SLOT: f32 = 0.1875; // ancho de la ventana: el simbolo no puede pasarse
const TAM_SIMBOLO: f32 = 0.20;  // tamano de fuente del simbolo, x lado del gabinete
// marquesina: el panel cian de arriba viene vacio a proposito
const MARQUESINA: [f32; 4] = [0.2422, 0.1406, 0.5156, 0.1016]; // x, y, w, h
const TEXTO_MARQUESINA: &str = "LA CASA";
const TAM_MARQUESINA: f32 = 0.62; // fraccion del alto del panel
const COLOR_MARQUESINA: Color = Color { r: 26, g: 20, b: 22, a: 255 }; // #1A1416
// HUD de la escena: va afuera del gabinete, nunca encima del arte
const MARGEN_HUD: i32 = 18;
const Y_PAGOS: i32 = 210;
const PAGOS: [&str; 6] = [
    "777   50",
    "DDD   30",
    "BBB   20",
    "CCC   15",
    "XXX  -10",
    "par    5",
];

// -------------------------------------------------- tipografia
// El mismo .ttf cargado en dos tamanos, horneados cerca de los tamanos que se
// dibujan de verdad: si el atlas se achica mucho la fuente pierde filas de
// pixeles y no se lee. Si no esta el archivo, todo cae a la fuente de raylib.
const RUTA_FUENTE: &str = "assets/fuentes/casino.ttf";

struct Fuentes {
    grande: Option<Font>,
    chica: Option<Font>,
}

impl Fuentes {
    fn para(&self, tam: i32) -> Option<&Font> {
        if tam >= 34 { self.grande.as_ref() } else { self.chica.as_ref() }
    }
}

fn espaciado(tam: i32) -> f32 {
    (tam as f32 / 12.0).max(1.0)
}

fn ancho_texto(dh: &RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, tam: i32) -> i32 {
    match f.para(tam) {
        Some(font) => font.measure_text(txt, tam as f32, espaciado(tam)).x as i32,
        None => dh.measure_text(txt, tam),
    }
}

fn dibujar_texto(dh: &mut RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, x: i32, y: i32, tam: i32, col: Color) {
    match f.para(tam) {
        Some(font) => dh.draw_text_ex(
            font, txt, Vector2::new(x as f32, y as f32), tam as f32, espaciado(tam), col,
        ),
        None => dh.draw_text(txt, x, y, tam, col),
    }
}

/// dibuja texto centrado en cx. Si no cabe a lo ancho de la pantalla baja el
/// tamano hasta que quepa, asi ninguna fuente rompe el layout.
fn texto_centrado(dh: &mut RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, cx: i32, y: i32, tam: i32, col: Color) {
    let max_w = ANCHO - 48;
    let mut t = tam;
    while t > 8 && ancho_texto(dh, f, txt, t) > max_w {
        t -= 2;
    }
    let w = ancho_texto(dh, f, txt, t);
    dibujar_texto(dh, f, txt, cx - w / 2, y, t, col);
}

/// devuelve (x, y, tam) para dibujar txt centrado en (cx, cy) sin pasarse de
/// max_w. Si no cabe baja el tamano: asi ningun simbolo se sale de su ventana.
fn ajustar_centrado(
    dh: &RaylibDrawHandle<'_>,
    f: &Fuentes,
    txt: &str,
    cx: f32,
    cy: f32,
    tam: i32,
    max_w: f32,
) -> (i32, i32, i32) {
    let mut t = tam;
    while t > 8 && ancho_texto(dh, f, txt, t) as f32 > max_w {
        t -= 1;
    }
    let w = ancho_texto(dh, f, txt, t) as f32;
    ((cx - w / 2.0) as i32, (cy - t as f32 / 2.0) as i32, t)
}

/// texto pegado a la derecha: x_der es el borde derecho, no el izquierdo
fn texto_derecha(dh: &mut RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, x_der: i32, y: i32, tam: i32, col: Color) {
    let w = ancho_texto(dh, f, txt, tam);
    dibujar_texto(dh, f, txt, x_der - w, y, tam, col);
}

// -------------------------------------------------- efectos de texto
// La fuente es una VHS/OSD, asi que los efectos van por ese lado: separacion
// de canales, glow y scanlines. Todo se dibuja con draw_text repetido, no hay
// shaders de por medio.

/// flotacion vertical suave, para que los titulos no se sientan pegados
fn flota(t: f32, vel: f32, amp: f32) -> i32 {
    ((t * vel).sin() * amp) as i32
}

/// texto con halo del mismo color alrededor
fn texto_glow(dh: &mut RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, x: i32, y: i32, tam: i32, col: Color) {
    let halo = Color { r: col.r, g: col.g, b: col.b, a: 45 };
    for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2), (-1, -1), (1, 1), (1, -1), (-1, 1)] {
        dibujar_texto(dh, f, txt, x + dx, y + dy, tam, halo);
    }
    dibujar_texto(dh, f, txt, x, y, tam, col);
}

fn texto_glow_centrado(dh: &mut RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, cx: i32, y: i32, tam: i32, col: Color) {
    let w = ancho_texto(dh, f, txt, tam);
    texto_glow(dh, f, txt, cx - w / 2, y, tam, col);
}

/// separacion de canales rojo/cyan con temblor, como cinta gastada
fn texto_vhs(dh: &mut RaylibDrawHandle<'_>, f: &Fuentes, txt: &str, cx: i32, y: i32, tam: i32, col: Color, t: f32) {
    let w = ancho_texto(dh, f, txt, tam);
    let x = cx - w / 2;
    let sep = 2 + ((t * 2.3).sin().abs() * 2.0) as i32;
    dibujar_texto(dh, f, txt, x - sep, y, tam, Color { r: 255, g: 40, b: 90, a: 130 });
    dibujar_texto(dh, f, txt, x + sep, y, tam, Color { r: 60, g: 230, b: 255, a: 130 });
    dibujar_texto(dh, f, txt, x, y, tam, col);
}

/// scanlines + banda de tracking que recorre la pantalla de arriba a abajo
fn efecto_vhs(dh: &mut RaylibDrawHandle<'_>, t: f32, alto: i32) {
    let mut y = 0;
    while y < alto {
        dh.draw_rectangle(0, y, ANCHO, 1, Color { r: 0, g: 0, b: 0, a: 40 });
        y += 3;
    }
    // la banda tarda ~8s en cruzar
    let banda = ((t * 0.125).fract() * (alto + 160) as f32) as i32 - 80;
    for i in 0..40 {
        let a = ((1.0 - i as f32 / 40.0) * 13.0) as u8;
        dh.draw_rectangle(0, banda + i, ANCHO, 1, Color { r: 255, g: 215, b: 240, a });
    }
}

/// multiplica el color por un factor de brillo, saturando en 255
fn atenuar(c: Color, f: f32) -> Color {
    Color {
        r: (c.r as f32 * f).clamp(0.0, 255.0) as u8,
        g: (c.g as f32 * f).clamp(0.0, 255.0) as u8,
        b: (c.b as f32 * f).clamp(0.0, 255.0) as u8,
        a: c.a,
    }
}

/// mezcla dos colores: t=0 devuelve a, t=1 devuelve b
fn mezclar(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: (a.r as f32 + (b.r as f32 - a.r as f32) * t) as u8,
        g: (a.g as f32 + (b.g as f32 - a.g as f32) * t) as u8,
        b: (a.b as f32 + (b.b as f32 - a.b as f32) * t) as u8,
        a: 255,
    }
}

/// fondo de alfombra de teatro: degrade vertical mas un tejido en rombo.
/// Va abajo de todo, y despues la vineta le come los bordes hasta negro.
fn alfombra(dh: &mut RaylibDrawHandle<'_>) {
    let hb = ALTO / ALFOMBRA_BANDAS + 1;
    for i in 0..ALFOMBRA_BANDAS {
        let t = i as f32 / (ALFOMBRA_BANDAS - 1) as f32;
        let c = atenuar(mezclar(ALFOMBRA_ALTO, ALFOMBRA, t), ALFOMBRA_BRILLO);
        dh.draw_rectangle(0, i * hb, ANCHO, hb, c);
    }

    // tejido: dos familias de diagonales cruzadas a 45 grados
    let paso = ANCHO as f32 * ROMBO_PASO;
    let atenuado = atenuar(ALFOMBRA_HILO, ALFOMBRA_BRILLO);
    let hilo = Color {
        r: atenuado.r,
        g: atenuado.g,
        b: atenuado.b,
        a: ROMBO_ALPHA,
    };
    let alto = ALTO as f32;
    let n = ((ANCHO as f32 + alto) / paso) as i32;
    for i in 0..n {
        let x = i as f32 * paso - alto;
        dh.draw_line_ex(Vector2::new(x, 0.0), Vector2::new(x + alto, alto), ROMBO_GROSOR, hilo);
        dh.draw_line_ex(Vector2::new(x, alto), Vector2::new(x + alto, 0.0), ROMBO_GROSOR, hilo);
    }
}

/// vineta: bandas concentricas que oscurecen hacia los bordes. Cubre toda la
/// ventana (no solo VIEW_H), asi lo saturado queda al medio y nada mas.
fn vineta(dh: &mut RaylibDrawHandle<'_>, alcance: f32, alpha_max: f32) {
    const BANDAS: i32 = 16;
    let hb = (ALTO as f32 * alcance / BANDAS as f32) as i32 + 1;
    let wb = (ANCHO as f32 * alcance / BANDAS as f32) as i32 + 1;
    for i in 0..BANDAS {
        let t = 1.0 - i as f32 / BANDAS as f32;
        let a = (t * t * alpha_max) as u8;
        let c = Color { r: 0, g: 0, b: 0, a };
        dh.draw_rectangle(0, i * hb, ANCHO, hb, c);
        dh.draw_rectangle(0, ALTO - (i + 1) * hb, ANCHO, hb, c);
        dh.draw_rectangle(i * wb, 0, wb, ALTO, c);
        dh.draw_rectangle(ANCHO - (i + 1) * wb, 0, wb, ALTO, c);
    }
}

/// grano de pelicula. Recorrer los 960x640 pixeles seria medio millon de
/// iteraciones por frame; con un puñado de rects sueltos se ve igual y sale
/// casi gratis. Usa el mismo rand que ya usan los rodillos.
fn grano(dh: &mut RaylibDrawHandle<'_>) {
    let mut rng = rand::thread_rng();
    let blanco = Color { r: 255, g: 255, b: 255, a: GRANO_ALPHA };
    for _ in 0..GRANO_RECTS {
        let x = rng.gen_range(0..ANCHO);
        let y = rng.gen_range(0..ALTO);
        dh.draw_rectangle(x, y, GRANO_PX, GRANO_PX, blanco);
    }
}

/// Color de cada simbolo. Antes eran todos blancos y los rodillos se veian
/// planos: ahora rosa los que pagan fuerte (y la calavera, que quema igual) y
/// cian el resto. Los que entraron en la combinacion que pago van a full, los
/// otros atenuados, asi el ganador se sigue leyendo de un vistazo.
fn color_simbolo(s: Simbolo, ganador: bool) -> Color {
    let base = match s {
        Simbolo::Siete | Simbolo::Diamante | Simbolo::Calavera => NEON,
        Simbolo::Cereza | Simbolo::Campana => CYAN,
    };
    if ganador { base } else { atenuar(base, 0.72) }
}

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
    let perseguidor_tex = rl.load_texture(&thread, "assets/sprites/sombra_sheet .png").ok();
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

    // el gabinete se carga una sola vez, no cada frame. Va con filtro POINT
    // porque es pixel art y en bilineal se ve borroso al escalarlo. Si falta el
    // archivo la escena cae a los rodillos de texto, no revienta.
    let gabinete = rl.load_texture(&thread, RUTA_GABINETE).ok();
    match &gabinete {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, la maquina va con los rodillos de texto",
            RUTA_GABINETE
        ),
    }

    // logo y placas de la bienvenida: misma historia, una sola carga y POINT
    let logo = rl.load_texture(&thread, RUTA_LOGO).ok();
    match &logo {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, el titulo va con la fuente",
            RUTA_LOGO
        ),
    }

    let fondo_maquina = rl.load_texture(&thread, RUTA_FONDO_MAQUINA).ok();
    match &fondo_maquina {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, la maquina va con fondo negro",
            RUTA_FONDO_MAQUINA
        ),
    }

    let marco = rl.load_texture(&thread, RUTA_MARCO).ok();
    match &marco {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, el marco va dibujado con rects",
            RUTA_MARCO
        ),
    }

    let placas = rl.load_texture(&thread, RUTA_PLACAS).ok();
    match &placas {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, las placas van dibujadas con rects",
            RUTA_PLACAS
        ),
    }

    let fuentes = if std::path::Path::new(RUTA_FUENTE).exists() {
        Fuentes {
            grande: rl.load_font_ex(&thread, RUTA_FUENTE, 64, None).ok(),
            chica: rl.load_font_ex(&thread, RUTA_FUENTE, 30, None).ok(),
        }
    } else {
        println!("aviso: no encontre {}, va la fuente default de raylib", RUTA_FUENTE);
        Fuentes { grande: None, chica: None }
    };

    let mut escena = Escena::Bienvenida;
    let mut est = Estado::nuevo(PISOS[0].mapa, PISOS[0].vel_enemigo);
    let mut usar_tex = true;
    let mut piso: usize = 0;
    let mut frase_muerte: usize = 0;
    let mut maq = Maquina::nueva(PISOS[0].cuota, PISOS[0].giros);
    let mut anim = AnimRodillos::nueva();
    let mut fundido = Fundido::nuevo();
    // de donde se entro a la maquina: false = la del arranque, true = una pared M del laberinto
    let mut maquina_en_laberinto = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let tiempo = rl.get_time() as f32; // reloj global para los efectos
        // el cambio de escena recien pasa cuando el velo llego a negro
        if let Some(e) = fundido.actualizar(dt) {
            escena = e;
        }
        match escena {
            // ============================================ BIENVENIDA
            Escena::Bienvenida => {
                // la seleccion mueve `piso`, que es el indice de PISOS: de ahi
                // sale el mapa, la cuota, los giros y la sombra
                let n_pisos = PISOS.len();
                if rl.is_key_pressed(KeyboardKey::KEY_A) || rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
                    piso = (piso + n_pisos - 1) % n_pisos;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_D) || rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
                    piso = (piso + 1) % n_pisos;
                }
                // atajo por numero, como estaba antes
                if rl.is_key_pressed(KeyboardKey::KEY_ONE) { piso = 0; }
                if rl.is_key_pressed(KeyboardKey::KEY_TWO) { piso = 1; }
                if rl.is_key_pressed(KeyboardKey::KEY_THREE) { piso = 2; }

                if !fundido.en_curso()
                    && (rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_SPACE)) {
                    // se arranca en la maquina, el laberinto recien si falla la cuota
                    let cfg = PISOS[piso];
                    maq = Maquina::nueva(cfg.cuota, cfg.giros);
                    anim = AnimRodillos::nueva();
                    maquina_en_laberinto = false;
                    fundido.ir_a(Escena::Maquina);
                }

                let cx = ANCHO / 2;
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(Color { r: 0, g: 0, b: 0, a: 255 });
                alfombra(&mut dh);

                // brillo global del letrero: el tubo esta sucio y parpadea
                let f = (0.85 + 0.15 * (tiempo * 7.0).sin() + 0.05 * (tiempo * 23.0).sin())
                    .clamp(FLICKER_MIN, FLICKER_MAX);

                // ---------------------------------------- marco art deco
                // el alto sale de la relacion del png, no se estira. Si no esta
                // la textura, el marco cae al rect de antes con sus medidas.
                let (mx, my, mw, mh) = match &marco {
                    Some(tex) => {
                        let w = ANCHO as f32 * MARCO_ANCHO;
                        let h = w / MARCO_RATIO;
                        let x = (ANCHO as f32 - w) / 2.0;
                        let y = ALTO as f32 * MARCO_Y;
                        dh.draw_texture_pro(
                            tex,
                            Rectangle::new(0.0, 0.0, MARCO_PX.0, MARCO_PX.1),
                            Rectangle::new(x, y, w, h),
                            Vector2::zero(), 0.0, atenuar(Color::WHITE, f),
                        );
                        (x, y, w, h)
                    }
                    None => {
                        let w = ANCHO as f32 * MARQ_W;
                        let h = ALTO as f32 * MARQ_H;
                        let x = (ANCHO as f32 - w) / 2.0;
                        let y = ALTO as f32 * MARQ_Y;
                        let rect = Rectangle::new(x, y, w, h);
                        dh.draw_rectangle_rec(rect, RELLENO_MARQ);
                        dh.draw_rectangle_lines_ex(rect, ANCHO as f32 * MARQ_BORDE, atenuar(GRANATE, f));
                        (x, y, w, h)
                    }
                };

                // ---------------------------------------- logo, adentro del hueco
                // se escala contra el ancho del hueco y se verifica que el alto
                // tambien entre; si no, manda el alto. Nunca se estira.
                let [hx, hy, hw, hh] = MARCO_HUECO;
                let hueco = Rectangle::new(
                    mx + mw * hx, my + mh * hy, mw * hw, mh * hh,
                );
                let mut logo_w = hueco.width * MARGEN_LOGO;
                let mut logo_h = logo_w / LOGO_RATIO;
                if logo_h > hueco.height * MARGEN_LOGO {
                    logo_h = hueco.height * MARGEN_LOGO;
                    logo_w = logo_h * LOGO_RATIO;
                }
                let logo_x = hueco.x + (hueco.width - logo_w) / 2.0;
                let logo_y = hueco.y + (hueco.height - logo_h) / 2.0;

                match &logo {
                    Some(tex) => {
                        dh.draw_texture_pro(
                            tex,
                            Rectangle::new(0.0, 0.0, LOGO_PX.0, LOGO_PX.1),
                            Rectangle::new(logo_x, logo_y, logo_w, logo_h),
                            Vector2::zero(), 0.0, atenuar(Color::WHITE, f),
                        );
                    }
                    None => {
                        // fallback: el titulo de antes, con la fuente VHS
                        let t1 = (ALTO as f32 * TAM_TITULO) as i32;
                        let t2 = (ALTO as f32 * TAM_SUBTITULO) as i32;
                        let gap = ALTO as f32 * GAP_TITULO;
                        let alto_bloque = t1 as f32 + gap + t2 as f32;
                        let y_bloque = hueco.y + (hueco.height - alto_bloque) / 2.0;
                        let ancho_util = hueco.width * MARGEN_LOGO;
                        let cx_hueco = hueco.x + hueco.width / 2.0;

                        let (x1, y1, tt1) = ajustar_centrado(&dh, &fuentes, "LA CASA",
                            cx_hueco, y_bloque + t1 as f32 / 2.0, t1, ancho_util);
                        texto_glow(&mut dh, &fuentes, "LA CASA", x1, y1, tt1, atenuar(NEON, f));

                        let (x2, y2, tt2) = ajustar_centrado(&dh, &fuentes, "SIEMPRE GANA",
                            cx_hueco, y_bloque + t1 as f32 + gap + t2 as f32 / 2.0,
                            t2, ancho_util);
                        texto_glow(&mut dh, &fuentes, "SIEMPRE GANA", x2, y2, tt2, atenuar(CYAN, f));
                    }
                }

                // ---------------------------------------- placas de piso, en fila
                // el alto se calcula, no se escribe: ancho / 2.5. Los huecos se
                // reparten parejo a los costados y entre placas, asi la fila
                // coincide con como se navega (A/D, izquierda a derecha).
                let pw = ANCHO as f32 * PLACA_ANCHO;
                let ph = pw / PLACA_RATIO;
                let hueco_p = (ANCHO as f32 - pw * PISOS.len() as f32) / (PISOS.len() + 1) as f32;
                let py0 = ALTO as f32 * PLACAS_Y;
                let pulso = 0.85 + 0.15 * (tiempo * 3.2).sin();
                let tam_placa = (ALTO as f32 * TAM_PLACA) as i32;

                for (i, cfg) in PISOS.iter().enumerate() {
                    let placa = Rectangle::new(hueco_p + (pw + hueco_p) * i as f32, py0, pw, ph);
                    let elegida = i == piso;

                    match &placas {
                        Some(tex) => {
                            // columna 0 = apagada, columna 1 = encendida
                            let col = if elegida { 1.0 } else { 0.0 };
                            let tinte = if elegida {
                                atenuar(Color::WHITE, pulso)
                            } else {
                                Color::WHITE
                            };
                            dh.draw_texture_pro(
                                tex,
                                Rectangle::new(col * PLACA_W, i as f32 * PLACA_H, PLACA_W, PLACA_H),
                                placa,
                                Vector2::zero(), 0.0, tinte,
                            );
                            // el nombre ya viene dibujado en la textura
                        }
                        None => {
                            // fallback: placa de rects, y ahi si va el nombre
                            let (relleno, borde, tinta) = if elegida {
                                (atenuar(LATON_TENUE, pulso), atenuar(LATON, pulso * f), atenuar(LATON, pulso))
                            } else {
                                (GRANATE_OSCURO, LATON_TENUE, atenuar(LATON, 0.7))
                            };
                            dh.draw_rectangle_rec(placa, relleno);
                            dh.draw_rectangle_lines_ex(placa, ANCHO as f32 * PLACA_BORDE, borde);

                            let (tx, ty, tt) = ajustar_centrado(&dh, &fuentes, cfg.nombre,
                                placa.x + placa.width / 2.0, placa.y + placa.height / 2.0,
                                tam_placa, placa.width * 0.8);
                            dibujar_texto(&mut dh, &fuentes, cfg.nombre, tx, ty, tt, tinta);
                        }
                    }
                }

                // ---------------------------------------- suciedad
                vineta(&mut dh, VINETA_BIENVENIDA.0, VINETA_BIENVENIDA.1);

                // ---------------------------------------- letra chica
                // Va DESPUES de la vineta: al bajar la marquesina estas dos
                // lineas quedaron dentro de la banda oscura de abajo y no se
                // leian. Son HUD, no escena, asi que no las tapa.
                let cfg = PISOS[piso];
                let chico = (ALTO as f32 * TAM_CHICO) as i32;
                texto_centrado(&mut dh, &fuentes,
                    &format!("cuota {}  -  {} giros", cfg.cuota, cfg.giros),
                    cx, (ALTO as f32 * INFO_Y) as i32, chico, LATON);
                texto_centrado(&mut dh, &fuentes, "A D  o  flechas   elegir piso        ENTER  entrar",
                    cx, (ALTO as f32 * CONTROLES_Y) as i32, chico, LATON_TENUE);

                grano(&mut dh);
                efecto_vhs(&mut dh, tiempo, ALTO);
                fundido.velo(&mut dh);
            }

            // ============================================ MAQUINA
            Escena::Maquina => {
                let termino_anim = anim.actualizar(dt);

                // si la maquina esta adentro del laberinto, el sujeto sigue caminando
                // mientras uno jala: cada giro cuesta tiempo real
                if maquina_en_laberinto {
                    est.perseguir(dt);
                    if est.atrapado {
                        frase_muerte = (est.anim_t * 1000.0) as usize % 5;
                        fundido.ir_a(Escena::Derrota);
                    }
                }

                // jalar la palanca: el RNG se resuelve aca, los rodillos lo revelan despues
                if !anim.activa() && !fundido.en_curso() && !maq.termino
                    && rl.is_key_pressed(KeyboardKey::KEY_F) {
                    let antes = maq.creditos;
                    maq.girar();
                    anim.iniciar(maq.rodillos, maq.creditos - antes);
                }

                // salir por voluntad propia
                if !anim.activa() && !fundido.en_curso() && rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    if !maquina_en_laberinto {
                        est = entrar_al_laberinto(&PISOS[piso]);
                    }
                    fundido.ir_a(Escena::Jugando);
                }

                // recien cuando la animacion termino de mostrar el resultado se decide
                if termino_anim && maq.termino {
                    if maq.gano() {
                        // pago la cuota: se salta el laberinto y pasa de ronda
                        let destino = siguiente_ronda(&mut piso, &mut maq, &mut anim, &mut maquina_en_laberinto);
                        fundido.ir_a(destino);
                    } else if !maquina_en_laberinto {
                        // se le acabaron los giros afuera: lo tiran al laberinto
                        est = entrar_al_laberinto(&PISOS[piso]);
                        fundido.ir_a(Escena::Jugando);
                    }
                    // adentro del laberinto se queda hasta que salga con ENTER
                }

                // ---------------------------------------- dibujo
                let cx = ANCHO / 2;
                let mostrar_res = anim.mostrando_resultado();
                let gano_giro = anim.gano_giro;

                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(Color { r: 0, g: 0, b: 0, a: 255 });

                // ---- el cuarto va primero, el gabinete se apoya encima
                if let Some(tex) = &fondo_maquina {
                    dh.draw_texture_pro(
                        tex,
                        Rectangle::new(0.0, 0.0, FONDO_MAQUINA_PX.0, FONDO_MAQUINA_PX.1),
                        Rectangle::new(0.0, 0.0, ANCHO as f32, ALTO as f32),
                        Vector2::zero(), 0.0, Color::WHITE,
                    );

                    // ojo de la camara: pulso lento e irregular, dos senos que
                    // no cierran entre si. Sin halo: se nota de reojo y nada mas.
                    let p = (0.45 + 0.35 * (tiempo * 1.6).sin() + 0.2 * (tiempo * 0.7).sin())
                        .clamp(0.15, 1.0);
                    let a = (p * 255.0) as u8;
                    let ox = (ANCHO as f32 * OJO_CAMARA.0) as i32;
                    let oy = (ALTO as f32 * OJO_CAMARA.1) as i32;
                    dh.draw_circle(ox, oy, OJO_RADIO,
                        Color { r: OJO_ROJO.r, g: OJO_ROJO.g, b: OJO_ROJO.b, a });
                    dh.draw_circle(ox, oy, OJO_NUCLEO,
                        Color { r: OJO_BRILLO.r, g: OJO_BRILLO.g, b: OJO_BRILLO.b, a });
                }

                // El gabinete manda el layout: si esta la textura los simbolos van
                // adentro de sus ventanas, si no caen a los rodillos de texto.
                let lado = LADO_GABINETE;
                let gx = (ANCHO as f32 - lado) / 2.0;
                let gy = (ALTO as f32 - lado) / 2.0;
                let hay_arte = gabinete.is_some();

                // (centro x, centro y, ancho util) de cada ventana y tamano del simbolo
                let (slots, tam_simbolo): ([(f32, f32, f32); 3], i32) = match &gabinete {
                    Some(tex) => {
                        dh.draw_texture_pro(
                            tex,
                            Rectangle::new(0.0, 0.0, GABINETE_PX, GABINETE_PX),
                            Rectangle::new(gx, gy, lado, lado),
                            Vector2::zero(), 0.0, Color::WHITE,
                        );

                        // marquesina: texto oscuro para que contraste contra el cian
                        let [mx, my, mw, mh] = MARQUESINA;
                        let (tx, ty, tt) = ajustar_centrado(
                            &dh, &fuentes, TEXTO_MARQUESINA,
                            gx + lado * (mx + mw / 2.0),
                            gy + lado * (my + mh / 2.0),
                            (lado * mh * TAM_MARQUESINA) as i32,
                            lado * mw * 0.9,
                        );
                        dibujar_texto(&mut dh, &fuentes, TEXTO_MARQUESINA, tx, ty, tt, COLOR_MARQUESINA);

                        let mut sl = [(0.0, 0.0, 0.0); 3];
                        for (e, c) in sl.iter_mut().zip(CENTROS_SLOT.iter()) {
                            *e = (gx + lado * c, gy + lado * CENTRO_Y_SLOT, lado * ANCHO_SLOT);
                        }
                        (sl, (lado * TAM_SIMBOLO) as i32)
                    }
                    None => {
                        // sin arte: los rodillos dibujados a mano de siempre
                        dh.draw_rectangle_lines(10, 10, ANCHO - 20, ALTO - 20, NEON);
                        dh.draw_rectangle_lines(18, 18, ANCHO - 36, ALTO - 36,
                            Color { r: 255, g: 110, b: 199, a: 120 });
                        texto_vhs(&mut dh, &fuentes, "LA CASA SIEMPRE GANA", cx,
                            46 + flota(tiempo, 1.5, 4.0), 46, NEON, tiempo);

                        const RW: i32 = 200;
                        const RH: i32 = 218;
                        const GAP: i32 = 34;
                        let rx0 = cx - (RW * 3 + GAP * 2) / 2;
                        let ry = 172;
                        let mut sl = [(0.0, 0.0, 0.0); 3];
                        for (i, e) in sl.iter_mut().enumerate() {
                            let rx = rx0 + i as i32 * (RW + GAP);
                            dh.draw_rectangle(rx, ry, RW, RH, RODILLO_BG);
                            dh.draw_rectangle_lines_ex(
                                Rectangle::new(rx as f32, ry as f32, RW as f32, RH as f32), 2.0, NEON,
                            );
                            *e = ((rx + RW / 2) as f32, (ry + RH / 2) as f32, RW as f32);
                        }
                        (sl, 120)
                    }
                };

                // simbolos de los rodillos, encima del fondo que toque
                for (i, (sx, sy, sw)) in slots.iter().enumerate() {
                    let s = anim.simbolos_visual[i];
                    // se pinta ganador solo cuando ya paro todo y el giro pago
                    let ganador = mostrar_res && gano_giro
                        && anim.simbolos_visual.iter().filter(|o| **o == s).count() >= 2;
                    // el simbolo flota apenas mientras el rodillo esta quieto
                    let bob = if anim.activa() { 0.0 } else { flota(tiempo + i as f32, 2.0, 3.0) as f32 };
                    let (tx, ty, tt) = ajustar_centrado(
                        &dh, &fuentes, s.letra(), *sx, *sy + bob, tam_simbolo, *sw,
                    );
                    // el halo va solo en los ganadores, si no todos se ven borrosos
                    if ganador {
                        texto_glow(&mut dh, &fuentes, s.letra(), tx, ty, tt, color_simbolo(s, true));
                    } else {
                        dibujar_texto(&mut dh, &fuentes, s.letra(), tx, ty, tt, color_simbolo(s, false));
                    }
                }

                // fondo negro con vineta: nada compite con el gabinete
                if hay_arte {
                    vineta(&mut dh, VINETA_MAQUINA.0, VINETA_MAQUINA.1);
                }

                // ---- HUD, siempre afuera del gabinete
                dibujar_texto(&mut dh, &fuentes, &format!("RONDA {} DE {}", piso + 1, PISOS.len()),
                    MARGEN_HUD, MARGEN_HUD, 26, TEXTO);
                texto_derecha(&mut dh, &fuentes, &format!("CREDITOS {} / {}", maq.creditos, maq.cuota),
                    ANCHO - MARGEN_HUD, MARGEN_HUD, 26, TEXTO);
                texto_derecha(&mut dh, &fuentes, &format!("GIROS {}", maq.giros_restantes),
                    ANCHO - MARGEN_HUD, MARGEN_HUD + 32, 26,
                    if maq.giros_restantes <= 1 { META } else { TEXTO });

                // tabla de pagos en la columna libre de la izquierda
                let tenue = Color { r: 178, g: 150, b: 160, a: 255 };
                dibujar_texto(&mut dh, &fuentes, "PAGOS", MARGEN_HUD, Y_PAGOS - 36, 24, TEXTO);
                for (i, linea) in PAGOS.iter().enumerate() {
                    dibujar_texto(&mut dh, &fuentes, linea, MARGEN_HUD, Y_PAGOS + i as i32 * 32, 24, tenue);
                }

                // ---- franja de abajo: resultado del giro o los controles
                let y_bajo = ALTO - 64;
                if mostrar_res {
                    texto_glow_centrado(&mut dh, &fuentes, &anim.resultado_texto,
                        cx, y_bajo + flota(tiempo, 3.0, 3.0), 32, anim.resultado_color);

                    if anim.bonus() && (anim.total * 9.0).sin() > 0.0 {
                        texto_glow_centrado(&mut dh, &fuentes, "BONUS!!", cx, ALTO - 28, 24, DORADO);
                    } else if anim.maldicion() {
                        texto_vhs(&mut dh, &fuentes, "MALDICION", cx, ALTO - 28, 24, META, tiempo * 3.0);
                    }
                } else if !anim.activa() {
                    // parpadeo suave de la instruccion
                    let p = 0.72 + 0.28 * (anim.total * 3.0).sin();
                    let salida = if maquina_en_laberinto { "ENTER volver" } else { "ENTER al laberinto" };
                    let hint = if maq.termino {
                        format!("sin giros     {}", salida)
                    } else {
                        format!("F jalar     {}", salida)
                    };
                    texto_centrado(&mut dh, &fuentes, &hint, cx, y_bajo, 28,
                        Color { r: TEXTO.r, g: TEXTO.g, b: TEXTO.b, a: (p * 255.0) as u8 });
                }

                // el sujeto se acerca mientras uno esta jugando: borde rojo latiendo
                if maquina_en_laberinto {
                    let d = est.dist_enemigo();
                    let alerta = PISOS[piso].dist_alerta;
                    if d < alerta {
                        let t = (1.0 - d / alerta).clamp(0.0, 1.0);
                        let pulso = 0.5 + 0.5 * (anim.total * 8.0).sin();
                        let a = (t * pulso * 255.0) as u8;
                        let rojo = Color { r: META.r, g: META.g, b: META.b, a };
                        for k in 0..4 {
                            dh.draw_rectangle_lines(10 - k, 10 - k, ANCHO - 20 + k * 2, ALTO - 20 + k * 2, rojo);
                        }
                        texto_derecha(&mut dh, &fuentes, "SE ACERCA", ANCHO - MARGEN_HUD, MARGEN_HUD + 64, 26, rojo);
                    }
                }

                efecto_vhs(&mut dh, tiempo, ALTO);
                fundido.velo(&mut dh);
            }

            // ============================================ JUGANDO
            Escena::Jugando => {
                // input
                if rl.is_key_down(KeyboardKey::KEY_LEFT) {
                    est.a -= VEL_GIRO * dt;
                }

                if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
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
                if rl.is_key_down(KeyboardKey::KEY_Q) || rl.is_key_down(KeyboardKey::KEY_A) {
                    est.avanzar(-lado.cos() * paso, -lado.sin() * paso);
                }
                if rl.is_key_down(KeyboardKey::KEY_E) || rl.is_key_down(KeyboardKey::KEY_D) {
                    est.avanzar(lado.cos() * paso, lado.sin() * paso);
                }

                est.perseguir(dt);

                if rl.is_key_pressed(KeyboardKey::KEY_M) {
                    est.modo3d = !est.modo3d;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_T) {
                    usar_tex = !usar_tex;
                }

                // pared M: se entra a la maquina sin perder la posicion en el laberinto
                let junto_a_maquina = hay_adyacente(&est.grid, est.x, est.y, 'M');
                if junto_a_maquina && !maq.termino && !fundido.en_curso()
                    && rl.is_key_pressed(KeyboardKey::KEY_F) {
                    maquina_en_laberinto = true;
                    fundido.ir_a(Escena::Maquina);
                }

                // transiciones
                if est.gano {
                    // sobrevivio el laberinto: pasa de ronda
                    let destino = siguiente_ronda(&mut piso, &mut maq, &mut anim, &mut maquina_en_laberinto);
                    fundido.ir_a(destino);
                }
                if est.atrapado {
                    frase_muerte = (est.anim_t * 1000.0) as usize % 5;
                    escena = Escena::Derrota;
                }

                // dibujo
                let fps = rl.get_fps();
                let dist_e = est.dist_enemigo();
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(BG);

                if est.modo3d {
                  let zbuffer = render_3d(&mut dh, &est, &texturas, tex_piso.as_ref(), tex_techo.as_ref(), usar_tex);
                  if est.persiguiendo {
                        render::render_sombra(&mut dh, &est, &zbuffer);
                  }
                    render_minimapa(&mut dh, &est);
                } else {
                    render_2d(&mut dh, &est);
                }

                // el apagon solo aplica cuando el sujeto anda suelto
                let alerta = PISOS[piso].dist_alerta;
                if est.persiguiendo && dist_e < alerta {
    let t = (1.0 - dist_e / alerta).clamp(0.0, 1.0);
    let radio = (1.0 - t) * 0.5;
    for band in 0..20 {
        let bt = band as f32 / 20.0;
        if bt > radio {
            let ba = ((bt - radio) / (1.0 - radio)).clamp(0.0, 1.0);
            let a = (ba * ba * 255.0) as u8;
            let h = (VIEW_H as f32 * 0.05) as i32 + 1;
            dh.draw_rectangle(0, band * h, ANCHO, h, Color { r: 0, g: 0, b: 0, a });
            dh.draw_rectangle(0, VIEW_H - (band + 1) * h, ANCHO, h, Color { r: 0, g: 0, b: 0, a });
            let w = (ANCHO as f32 * 0.05) as i32 + 1;
            dh.draw_rectangle(band * w, 0, w, VIEW_H, Color { r: 0, g: 0, b: 0, a });
            dh.draw_rectangle(ANCHO - (band + 1) * w, 0, w, VIEW_H, Color { r: 0, g: 0, b: 0, a });
        }
    } // <-- cierra el for

    // esto va AFUERA del for
    let black_a = (t * t * t * 220.0) as u8;
    dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 0, g: 0, b: 0, a: black_a });

    if t > 0.6 {
        let flicker = ((est.anim_t * 12.0).sin() * (est.anim_t * 37.0).sin()).abs();
        let fa = (flicker * t * 180.0) as u8;
        dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 0, g: 0, b: 0, a: fa });
    }
}

                // aviso de que hay una maquina al lado
                if junto_a_maquina {
                    if maq.termino {
                        texto_centrado(&mut dh, &fuentes, "la maquina ya no te da mas", ANCHO / 2, VIEW_H - 52, 26,
                            Color { r: 170, g: 140, b: 150, a: 255 });
                    } else {
                        let p = 0.72 + 0.28 * (est.anim_t * 3.0).sin();
                        texto_centrado(&mut dh, &fuentes, "F jalar", ANCHO / 2, VIEW_H - 54, 30,
                            Color { r: NEON.r, g: NEON.g, b: NEON.b, a: (p * 255.0) as u8 });
                    }
                }

                dh.draw_rectangle(0, VIEW_H, ANCHO, HUD_H, Color { r: 16, g: 7, b: 30, a: 255 });

                let modo = if est.modo3d { "3D" } else { "2D" };
                dibujar_texto(&mut dh, &fuentes,
                    &format!("[{}]  ronda {}/{}  -  llega a la salida", modo, piso + 1, PISOS.len()),
                    12, VIEW_H + 8, 24, TEXTO,
                );
                // la distancia al sujeto solo cuando de verdad anda suelto
                let info = if est.persiguiendo {
                    format!("el sujeto a {:.1}   {} fps", dist_e, fps)
                } else {
                    format!("{} fps", fps)
                };
                dibujar_texto(&mut dh, &fuentes,
                    &info, ANCHO - 260, VIEW_H + 8, 24,
                    if est.persiguiendo && dist_e < alerta { ENEMIGO }
                    else { Color { r: 140, g: 110, b: 180, a: 255 } },
                );
                fundido.velo(&mut dh);
            }



    



            // ============================================ VICTORIA
            Escena::Victoria => {
                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    piso = 0;
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
                texto_vhs(&mut dh, &fuentes, "ESCAPASTE", ANCHO / 2,
                    VIEW_H / 2 + 20 + flota(tiempo, 1.6, 5.0), 58, NEON, tiempo);
                texto_centrado(&mut dh, &fuentes, "R   volver al menu", ANCHO / 2, VIEW_H / 2 + 96, 26, TEXTO);
                efecto_vhs(&mut dh, tiempo, ALTO);
            }

            // ============================================ DERROTA
            Escena::Derrota => {
                const FRASES: [&str; 5] = [
                    "La ambicion mata",
                    "La casa siempre gana",
                    "Debiste parar cuando podias",
                    "Nadie sale de aqui",
                    "El juego estaba arreglado",
                ];

                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    piso = 0;
                    escena = Escena::Bienvenida;
                }

                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(Color { r: 2, g: 0, b: 0, a: 255 });

                texto_vhs(&mut dh, &fuentes, FRASES[frase_muerte], ANCHO / 2,
                    VIEW_H / 2 - 30 + flota(tiempo, 1.1, 4.0), 50, META, tiempo * 2.0);
                texto_centrado(&mut dh, &fuentes, "R   volver al menu", ANCHO / 2, VIEW_H / 2 + 60, 26, TEXTO);
                efecto_vhs(&mut dh, tiempo, ALTO);
            }
        }
    }
}
