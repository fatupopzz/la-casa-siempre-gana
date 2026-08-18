use rand::Rng;
use raylib::prelude::*;
mod mapa;
mod raycast;
mod estado;
mod render;
mod juego;
mod maquina;
mod audio;
mod gen;
use audio::Audio;
use maquina::{AnimRodillos, FaseRodillos, Maquina, Palanca, Simbolo, N_SIMBOLOS};
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

// -------------------------------------------------- control
// El control es entrada ALTERNATIVA: se suma al teclado, no lo reemplaza. Todo
// lo que se hacia con teclas se sigue haciendo igual, y las dos entradas andan
// a la vez sin pisarse.
const GAMEPAD: i32 = 0; // solo el primer control, el juego es de a uno
/// Zona muerta de los sticks. Los analogicos nunca descansan en cero exacto:
/// sin esto el jugador camina y gira solo con el control quieto.
const ZONA_MUERTA: f32 = 0.20;

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

// Las tres velocidades estan DEBAJO de VEL (3.2), al reves de como estaba.
// Motivo: los pasillos miden una celda y DIST_ATRAPA es 0.5, asi que no se le
// puede pasar por al lado. Con la sombra mas rapida, cruzartela de frente era
// muerte segura sin importar como jugaras. Ahora podes retroceder y buscar otra
// rama. Le sacas 1.3 / 1.05 / 0.8 por segundo, que es el margen para perderte:
// el jugador promedio no encuentra la ruta buena a la primera y necesita poder
// equivocarse de pasillo sin que eso sea la muerte.
//
// dist_alerta tambien bajo: es el radio del apagon, y con 6.0 en un mapa de
// 11x11 la sombra te cegaba desde media pantalla de distancia. Ahora tiene que
// estar de verdad encima para taparte la vista.
const PISOS: [ConfigPiso; 3] = [
    ConfigPiso {
        nombre: "PISO 1",
        mapa: "mapas/piso1.txt",
        cuota: 65,
        giros: 20,
        vel_enemigo: 1.9,
        dist_alerta: 4.5,
    },
    ConfigPiso {
        nombre: "PISO 2",
        mapa: "mapas/piso2.txt",
        cuota: 70,
        giros: 16,
        vel_enemigo: 2.15,
        dist_alerta: 4.0,
    },
    ConfigPiso {
        nombre: "PISO 3",
        mapa: "mapas/piso3.txt",
        // 55 para que la rampa quede monotona: 44.4% / 20.7% / 19.0% de exito,
        // cada piso mas duro que el anterior. A 70 no daba, el 76% de los que
        // pagaban dependia de un triple 7 (que sale 9% de las veces en 12
        // giros); a 55 el 55% la pega sin ver un 7. Necesita p81, o sea que
        // NO pasa el chequeo p75: la monotonia se eligio por encima de eso.
        cuota: 55,
        giros: 12,
        vel_enemigo: 2.4,
        dist_alerta: 3.5,
    },
];

// Ventaja por meterse al laberinto por decision propia en vez de que te tiren
// adentro por quedarte sin giros. Sin esto elegir correr no cambiaba nada y era
// siempre la peor jugada: mismo laberinto, pero habiendo gastado los giros.
// Sigue sin poder escaparsele corriendo, la sombra queda arriba de VEL (3.2).
/// Segundos antes de que la sombra salga de la meta. En la entrada forzada va
/// en cero a proposito: si se queda quieta en la salida te espera ahi, que era
/// justo el problema. Saliendo de una, te la cruzas a mitad de camino y despues
/// la salida queda libre.
const RETRASO_SOMBRA: f32 = 0.0;
const VENTAJA_RETRASO: f32 = 1.5; // lo que suma entrar por decision propia
const VENTAJA_VEL: f32 = 0.15;    // cuanto mas lenta va la sombra
/// Piso duro: por mas ventaja que tenga, la sombra nunca baja de aca.
///
/// OJO, esto valia VEL + 0.15 (3.35) de cuando las velocidades rondaban 3.45 y
/// la ventaja podia dejarla mas lenta que el jugador. Con las velocidades de
/// hoy (1.9 / 2.15 / 2.4, todas DEBAJO de VEL) ese piso quedaba ARRIBA de la
/// velocidad base, asi que el .max() lo aplicaba siempre y salir por decision
/// propia ponia la sombra en 3.35 — mas rapida que el jugador y mucho mas
/// rapida que si te tiraban adentro. La "ventaja" era en realidad un castigo.
///
/// Ahora es un piso absoluto y bajo: nunca llega a aplicarse con las
/// velocidades actuales, y esta solo para que un ConfigPiso muy lento no deje
/// una sombra que no camina.
const VEL_SOMBRA_MIN: f32 = 1.2;
/// Techo del modo infinito. La sombra escala piso a piso pero nunca llega a
/// VEL: si igualara al jugador, correr en linea recta dejaria de ser una
/// salida y el modo se volveria imposible en vez de dificil.
const VEL_SOMBRA_MAX_INF: f32 = VEL - 0.1;

/// arranca el laberinto de un piso: su mapa, la sombra suelta y a la velocidad
/// que le toca. Todo sale del ConfigPiso, no hay consts de por medio.
///
/// `voluntario` = el jugador eligio salir en vez de quemar los giros. Se lleva
/// unos segundos antes de que la sombra salga y la sombra va algo mas lenta.
fn entrar_al_laberinto(cfg: &ConfigPiso, voluntario: bool) -> Estado {
    preparar_entrada(
        Estado::nuevo(cfg.mapa, vel_de_entrada(cfg.vel_enemigo, voluntario)),
        voluntario,
    )
}

/// Lo mismo pero para un piso del modo infinito, cuyo mapa se genero en runtime
/// y no vive en ningun archivo.
fn entrar_generado(inf: &Infinito, voluntario: bool) -> Estado {
    preparar_entrada(
        Estado::de_texto(&inf.mapa, vel_de_entrada(inf.vel_enemigo, voluntario)),
        voluntario,
    )
}

/// La ventaja por entrar por decision propia nunca puede dejar a la sombra MAS
/// RAPIDA que si no la hubieras tenido: por eso el piso duro se capa contra la
/// velocidad base en vez de aplicarse tal cual. Sin ese min(), un piso con base
/// por debajo de VEL_SOMBRA_MIN sale acelerado justo cuando elegis correr.
fn vel_de_entrada(base: f32, voluntario: bool) -> f32 {
    if voluntario {
        (base - VENTAJA_VEL).max(VEL_SOMBRA_MIN.min(base))
    } else {
        base
    }
}

fn preparar_entrada(mut est: Estado, voluntario: bool) -> Estado {
    est.modo3d = true;
    // La sombra sale por la puerta por la que entraste, no aparece adelante.
    // Hasta que salga no persigue ni te ciega; el que se queda quieto la ve
    // salir encima suyo.
    est.t_espera = if voluntario {
        RETRASO_SOMBRA + VENTAJA_RETRASO
    } else {
        RETRASO_SOMBRA
    };
    // con espera en cero tiene que arrancar persiguiendo ya: si no, el contador
    // nunca la despierta y la sombra se queda clavada en la salida
    est.persiguiendo = est.t_espera <= 0.0;
    est
}

// ============================================================ MODO INFINITO

/// Indice de la opcion "Infinito" en la fila de la bienvenida: va despues de
/// los tres pisos fijos. `piso` puede valer esto, asi que NADIE debe indexar
/// PISOS con `piso` a secas — para eso estan cuota_de() y compania.
const INFINITO: usize = PISOS.len();

/// Piso base del modo infinito. ~20x20 de grid sale de 10 celdas: el generador
/// da 2*celdas+1 por lado, o sea 21x21.
const INF_CELDAS: usize = 10;
/// Cuanto crece el laberinto por piso, EN CELDAS. El pedido era +5 filas y
/// columnas de grid, pero el grid siempre mide 2*celdas+1: solo puede crecer de
/// a 2. +2 celdas = +4 de grid, que es el escalon valido mas cercano a 5 por
/// abajo. Si se quiere el otro lado del redondeo, aca va un 3 (+6 de grid).
const INF_CRECE_CELDAS: usize = 2;
const INF_CUOTA_EXTRA: i32 = 10;
const INF_VEL_EXTRA: f32 = 0.15;

/// Un piso del modo infinito. Es lo que en los pisos fijos hace ConfigPiso,
/// salvo que el mapa es texto generado en vez de una ruta a un .txt, y que los
/// numeros salen de escalar el piso anterior en vez de estar escritos a mano.
struct Infinito {
    /// numero de piso, 1-based, el que se muestra en el HUD
    n: i32,
    celdas: usize,
    cuota: i32,
    giros: i32,
    vel_enemigo: f32,
    dist_alerta: f32,
    mapa: String,
}

impl Infinito {
    /// El primer piso arranca con la cuota y los giros del piso 1 fijo: la
    /// entrada al modo tiene que sentirse igual de dura que empezar el juego,
    /// la rampa viene despues.
    fn primero() -> Self {
        Infinito {
            n: 1,
            celdas: INF_CELDAS,
            cuota: PISOS[0].cuota,
            giros: PISOS[0].giros,
            vel_enemigo: PISOS[0].vel_enemigo,
            dist_alerta: PISOS[0].dist_alerta,
            mapa: gen::generar(INF_CELDAS, INF_CELDAS, 2),
        }
    }

    /// El piso siguiente: mas grande, mas caro y con la sombra mas rapida.
    ///
    /// La velocidad se capa contra VEL_SOMBRA_MAX_INF y no crece mas alla, asi
    /// que a partir de cierto piso lo unico que sigue escalando es el tamano
    /// del laberinto. Es a proposito: una sombra mas rapida que el jugador hace
    /// el modo imposible, un laberinto mas grande solo lo hace mas largo.
    ///
    /// Los giros NO escalan. Con la cuota subiendo +10 por piso, la maquina se
    /// vuelve inalcanzable sola y el modo se apoya en escaparse del laberinto,
    /// que es el bucle que de verdad se puede sostener para siempre.
    fn siguiente(&self) -> Self {
        let celdas = self.celdas + INF_CRECE_CELDAS;
        Infinito {
            n: self.n + 1,
            celdas,
            cuota: self.cuota + INF_CUOTA_EXTRA,
            giros: self.giros,
            vel_enemigo: (self.vel_enemigo + INF_VEL_EXTRA).min(VEL_SOMBRA_MAX_INF),
            dist_alerta: self.dist_alerta,
            mapa: gen::generar(celdas, celdas, 2),
        }
    }

    fn nombre(&self) -> String {
        format!("PISO {}", self.n)
    }
}

// Los cuatro de abajo son el unico camino permitido para leer los parametros
// del piso que se esta jugando: en modo infinito salen de Infinito y en los
// pisos fijos de PISOS, y asi ningun call site tiene que acordarse de cual es.

fn cuota_de(inf: &Option<Infinito>, piso: usize) -> i32 {
    inf.as_ref().map_or_else(|| PISOS[piso].cuota, |i| i.cuota)
}

fn giros_de(inf: &Option<Infinito>, piso: usize) -> i32 {
    inf.as_ref().map_or_else(|| PISOS[piso].giros, |i| i.giros)
}

fn alerta_de(inf: &Option<Infinito>, piso: usize) -> f32 {
    inf.as_ref()
        .map_or_else(|| PISOS[piso].dist_alerta, |i| i.dist_alerta)
}

fn nombre_de(inf: &Option<Infinito>, piso: usize) -> String {
    inf.as_ref()
        .map_or_else(|| PISOS[piso].nombre.to_string(), |i| i.nombre())
}

/// cuantos rodillos ya frenaron, leyendo solo la fase. Vive aca y no en
/// maquina.rs para no tocar la logica de la animacion: main solo necesita
/// saber cuando suena el golpe de un rodillo.
fn rodillos_parados(f: FaseRodillos) -> usize {
    match f {
        FaseRodillos::Girando(_) => 0,
        FaseRodillos::Parando(i, _) => i + 1,
        _ => 3,
    }
}

// -------------------------------------------------- lectura del control
// Los tres devuelven el valor neutro (0.0 / false) cuando no hay control
// conectado, asi que el que llama nunca tiene que preguntar si hay uno. Sin
// gamepad el juego corre exactamente igual que antes.

/// Lee un eje con zona muerta. Reescala lo que sobra del umbral a [0, 1] para
/// que apenas se pasa la zona muerta el movimiento arranque en cero y no pegue
/// un salto a ZONA_MUERTA. Conserva el signo, asi que sigue siendo analogico:
/// medio stick es media velocidad.
fn eje(rl: &RaylibHandle, axis: GamepadAxis) -> f32 {
    if !rl.is_gamepad_available(GAMEPAD) {
        return 0.0;
    }
    let v = rl.get_gamepad_axis_movement(GAMEPAD, axis);
    if v.abs() < ZONA_MUERTA {
        return 0.0;
    }
    let t = (v.abs() - ZONA_MUERTA) / (1.0 - ZONA_MUERTA);
    t.clamp(0.0, 1.0) * v.signum()
}

/// Boton apretado en ESTE frame, para lo que va de a un disparo (elegir piso,
/// confirmar, jalar). El equivalente de is_key_pressed.
fn boton(rl: &RaylibHandle, b: GamepadButton) -> bool {
    rl.is_gamepad_available(GAMEPAD) && rl.is_gamepad_button_pressed(GAMEPAD, b)
}

/// Cualquier boton, para las pantallas finales donde alcanza con "apreta lo que
/// sea" y no importa cual.
fn boton_cualquiera(rl: &RaylibHandle) -> bool {
    rl.is_gamepad_available(GAMEPAD) && rl.get_gamepad_button_pressed().is_some()
}

// -------------------------------------------------- apagon del perseguidor
// Cuanto te ciega la sombra al acercarse. Estaba en velo de 220 con bandas casi
// cerradas y parpadeo de 180: quedabas practicamente ciego justo cuando tenias
// que esquivarla, que ahora es la jugada principal. La idea es que se sienta
// que se acerca, no que te apaguen la pantalla.
const APAGON_VELO: f32 = 85.0;     // velo negro maximo, sobre 255
const APAGON_BANDAS: f32 = 140.0;  // cuanto tapan las bandas de los bordes
const APAGON_FLICKER: f32 = 45.0;  // el parpadeo de cuando la tiene encima
/// hasta donde llegan a cerrar las bandas: 0.30 deja el centro siempre limpio
const APAGON_CENTRO: f32 = 0.30;

// -------------------------------------------------- fundido entre escenas
const T_FUNDIDO: f32 = 0.25; // dura lo mismo cerrar que abrir
/// Al fallar la cuota la pantalla se queda en negro y en silencio antes de
/// tirarte al laberinto. Ese vacio es el efecto; no lleva sonido encima.
const T_PAUSA_FALLO: f32 = 1.0;

/// Fundido a negro para entrar y salir de la maquina: el corte de 3D a 2D a
/// pantalla completa se siente brusco sin esto. Primero cierra a negro, ahi
/// cambia la escena, y despues abre. El cambio no pasa hasta que la pantalla
/// esta negra del todo, asi no se ve el salto.
struct Fundido {
    t: f32,
    destino: Option<Escena>,
    /// cuanto se queda en negro antes de cambiar de escena. Se usa al fallar la
    /// cuota: ese silencio en negro es el efecto, no hay que taparlo con nada.
    t_pausa: f32,
}

impl Fundido {
    fn nuevo() -> Self {
        Fundido { t: 0.0, destino: None, t_pausa: 0.0 }
    }

    /// pide el cambio de escena. Si ya hay uno en curso no lo pisa. El reloj
    /// NO se reinicia: si venia abriendo, cierra desde donde estaba el velo en
    /// vez de saltar a transparente y volver a oscurecer.
    fn ir_a(&mut self, e: Escena) {
        self.ir_a_con_pausa(e, 0.0);
    }

    /// igual que ir_a pero aguantando `pausa` segundos en negro antes del cambio
    fn ir_a_con_pausa(&mut self, e: Escena, pausa: f32) {
        if self.destino.is_none() {
            self.destino = Some(e);
            self.t_pausa = pausa;
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
                if self.t_pausa > 0.0 {
                    // se queda en negro y en silencio antes de cambiar
                    self.t_pausa -= dt;
                } else {
                    return self.destino.take();
                }
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
// Grano de pelicula del post-proceso. Se genera con ruido blanco en vez de
// venir de un png: es ruido, no arte, y generarlo evita otro asset que se
// pueda perder. Mas grande que ANCHO x VIEW_H (960x600) a proposito, para que
// el recorte de cada cuadro se mueva dentro del sobrante sin envolver.
const GRANO_W: i32 = 1216;
const GRANO_H: i32 = 896;
/// Fraccion de pixeles blancos. Bajo: el grano tiene que ensuciar, no nevar.
const GRANO_FACTOR: f32 = 0.12;

// misma hoja pero de una sola fila (800x160): la placa del modo infinito
const RUTA_PLACA_INF: &str = "assets/sprites/placa_infinito.png";
const PLACA_ANCHO: f32 = 0.28;  // fraccion de ANCHO -> 269x108
/// Cuanto de la pantalla puede ocupar la fila de placas sumada. Lo que sobra se
/// reparte como huecos, contando tambien los dos de las puntas.
const FILA_UTIL: f32 = 0.88;
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
// la alfombra se ve, pero sigue MUY por debajo del logo: a este brillo su
// punto mas claro queda en (25,7,9) contra el (255,110,199) del rosa
const ALFOMBRA_BRILLO: f32 = 0.55;

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
const ANCHO_SLOT: f32 = 0.1875; // ancho de la ventana: el simbolo no puede pasarse
const Y_SLOT: f32 = 0.3672;     // tope de la ventana negra
const ALTO_SLOT: f32 = 0.3047;  // alto de la ventana negra
const TAM_SIMBOLO: f32 = 0.20;  // tamano de fuente del simbolo, solo para el fallback

// tira de simbolos: 128x640, 5 celdas de 128x128, en el orden del enum Simbolo
const RUTA_SIMBOLOS: &str = "assets/sprites/simbolos.png";
const CELDA_SIMBOLO: f32 = 128.0;

// palanca. Va normalizada sobre el gabinete, como los slots.
const RUTA_PALANCA: &str = "assets/sprites/palanca.png";
const PALANCA_PX: (f32, f32) = (72.0, 246.0);
const PALANCA_X: f32 = 0.8906;
const PALANCA_Y: f32 = 0.3047;
const PALANCA_W: f32 = 0.0938;
const PALANCA_H: f32 = 0.3203;
const PALANCA_RECORRIDO: f32 = 0.0703; // cuanto baja, en fracciones del lado
// marquesina: el panel cian de arriba viene vacio a proposito
const MARQUESINA: [f32; 4] = [0.2422, 0.1406, 0.5156, 0.1016]; // x, y, w, h
const TEXTO_MARQUESINA: &str = "LA CASA";
const TAM_MARQUESINA: f32 = 0.62; // fraccion del alto del panel
const COLOR_MARQUESINA: Color = Color { r: 26, g: 20, b: 22, a: 255 }; // #1A1416
// HUD de la escena: va afuera del gabinete, nunca encima del arte
const MARGEN_HUD: i32 = 18;
const T_CONTEO: f32 = 0.35; // cuanto tarda el contador en subir hasta el pago
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
// La firma queda larga a proposito: todos los helpers de texto de aca abajo
// tienen la misma forma (dh, fuentes, texto, posicion, tamano, color) y este
// solo suma el reloj. Agruparlos en un struct rompe el parecido entre ellos.
#[allow(clippy::too_many_arguments)]
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

    let simbolos_tex = rl.load_texture(&thread, RUTA_SIMBOLOS).ok();
    match &simbolos_tex {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, los rodillos van con letras",
            RUTA_SIMBOLOS
        ),
    }

    let palanca_tex = rl.load_texture(&thread, RUTA_PALANCA).ok();
    match &palanca_tex {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!("aviso: no encontre {}, va sin palanca", RUTA_PALANCA),
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

    // El grano se arma una sola vez: generar ruido por cuadro costaria una
    // imagen entera cada frame. Si la carga falla queda en None y render_post
    // simplemente no dibuja esa capa — el juego no se cae por un efecto.
    let grano_tex = {
        let img = Image::gen_image_white_noise(GRANO_W, GRANO_H, GRANO_FACTOR);
        rl.load_texture_from_image(&thread, &img).ok()
    };
    match &grano_tex {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!("aviso: no pude crear la textura de grano, el post va sin ella"),
    }

    let placa_inf = rl.load_texture(&thread, RUTA_PLACA_INF).ok();
    match &placa_inf {
        Some(tex) => tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT),
        None => eprintln!(
            "aviso: no encontre {}, la placa del infinito va dibujada con rects",
            RUTA_PLACA_INF
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

    // El dispositivo de audio se queda aca a proposito: Music y Sound le
    // prestan referencia, asi que tiene que vivir tanto como el bucle. Si no
    // abre, Audio::mudo() deja todo en None y el juego corre en silencio.
    let dispositivo = RaylibAudio::init_audio_device().ok();
    if dispositivo.is_none() {
        eprintln!("aviso: no pude abrir el dispositivo de audio, el juego va en silencio");
    }
    let mut audio = match &dispositivo {
        Some(d) => Audio::nuevo(d),
        None => Audio::mudo(),
    };

    let mut escena = Escena::Bienvenida;
    let mut est = Estado::nuevo(PISOS[0].mapa, PISOS[0].vel_enemigo);
    let mut usar_tex = true;
    // indice de la opcion elegida: 0..2 son los pisos fijos, INFINITO es la cuarta
    let mut piso: usize = 0;
    // Some(..) = se esta jugando el modo infinito. Lo fija cada ENTER de la
    // bienvenida, a Some o a None segun que placa estuviera elegida, asi que
    // vale para toda la partida. Mientras es Some, los parametros del piso
    // salen de aca y no de PISOS: ese es el invariante que deja que `piso`
    // valga INFINITO sin que nadie indexe PISOS fuera de rango.
    let mut inf: Option<Infinito> = None;
    // lo prende la transicion que cierra un piso del modo infinito, y se
    // atiende cuando el velo llega a negro. Ver el bloque de abajo.
    let mut avanzar_infinito = false;
    let mut frase_muerte: usize = 0;
    let mut maq = Maquina::nueva(PISOS[0].cuota, PISOS[0].giros);
    let mut anim = AnimRodillos::nueva();
    let mut fundido = Fundido::nuevo();
    let mut palanca = Palanca::nueva();
    // El contador de creditos que se DIBUJA va aparte de maq.creditos, porque
    // maq.girar() suma en el mismo frame del jalon: si el HUD leyera eso
    // directo, cantaria el resultado antes de que paren los rodillos.
    let mut cred_objetivo = 0;
    let mut cred_vista = 0.0f32;
    // de donde se entro a la maquina: false = la del arranque, true = una pared M del laberinto
    let mut maquina_en_laberinto = false;
    // de que ruta se salio: true = jalo ENTER y se fue, false = se quedo sin giros
    let mut salio_voluntario = false;
    // pego la cuota desde una 'M' de adentro: la sombra se apago pero todavia
    // falta caminar hasta la 'B'. Cambia el final que se dispara al pisarla.
    let mut pago_cuota = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let tiempo = rl.get_time() as f32; // reloj global para los efectos
        // el stream de musica se rellena cada frame, si no se corta a los pocos segundos
        audio.actualizar(dt);
        // el cambio de escena recien pasa cuando el velo llego a negro
        if let Some(e) = fundido.actualizar(dt) {
            escena = e;

            // ---- cierre de piso del modo infinito
            // Justo aca y no al pedir la transicion: mientras el velo cierra,
            // la escena vieja se sigue dibujando, y si el piso ya hubiera
            // avanzado el HUD cantaria el numero nuevo con el jugador todavia
            // parado en el laberinto anterior. Con la pantalla en negro no se
            // ve nada de eso, y ademas es el momento natural para pagar la
            // generacion del laberinto, que es lo mas caro del pase.
            //
            // Deja la partida como recien entrada: maquina nueva, sin cuota
            // pagada y sin arrastrar de donde se venia.
            if avanzar_infinito {
                avanzar_infinito = false;
                if let Some(actual) = &inf {
                    let sig = actual.siguiente();
                    maq = Maquina::nueva(sig.cuota, sig.giros);
                    inf = Some(sig);
                    anim = AnimRodillos::nueva();
                    maquina_en_laberinto = false;
                    salio_voluntario = false;
                    pago_cuota = false;
                    cred_vista = 0.0;
                }
            }
        }
        match escena {
            // ============================================ BIENVENIDA
            Escena::Bienvenida => {
                audio.bienvenida();

                // la seleccion mueve `piso`: 0..2 son los indices de PISOS y la
                // cuarta es INFINITO, que no tiene entrada en PISOS y de ahi
                // que la fila se recorra hasta n_opciones y no hasta PISOS.len()
                let n_opciones = PISOS.len() + 1;
                if rl.is_key_pressed(KeyboardKey::KEY_A) || rl.is_key_pressed(KeyboardKey::KEY_LEFT)
                    || boton(&rl, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                    piso = (piso + n_opciones - 1) % n_opciones;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_D) || rl.is_key_pressed(KeyboardKey::KEY_RIGHT)
                    || boton(&rl, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                    piso = (piso + 1) % n_opciones;
                }
                // atajo por numero, como estaba antes
                if rl.is_key_pressed(KeyboardKey::KEY_ONE) { piso = 0; }
                if rl.is_key_pressed(KeyboardKey::KEY_TWO) { piso = 1; }
                if rl.is_key_pressed(KeyboardKey::KEY_THREE) { piso = 2; }
                if rl.is_key_pressed(KeyboardKey::KEY_FOUR) { piso = INFINITO; }

                if !fundido.en_curso()
                    && (rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_SPACE)
                        || boton(&rl, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT)) {
                    // se fija en cada entrada, no solo al elegir infinito: si
                    // se vuelve al menu despues de una partida infinita, elegir
                    // un piso fijo tiene que apagarlo. El primer laberinto se
                    // genera aca mismo, en el frame que arranca el fundido.
                    inf = if piso == INFINITO {
                        Some(Infinito::primero())
                    } else {
                        None
                    };
                    // se arranca en la maquina, el laberinto recien si falla la cuota
                    maq = Maquina::nueva(cuota_de(&inf, piso), giros_de(&inf, piso));
                    anim = AnimRodillos::nueva();
                    maquina_en_laberinto = false;
                    salio_voluntario = false;
                    pago_cuota = false;
                    cred_vista = 0.0; // si no, el contador viene bajando desde la partida anterior
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
                // Con la cuarta placa la fila ya no entra a PLACA_ANCHO cada
                // una (4 x 0.28 = 1.12 de pantalla), asi que el ancho es el
                // menor entre el de siempre y lo que deja FILA_UTIL repartido.
                // Con tres opciones gana PLACA_ANCHO y la fila queda igual que
                // antes; es la cuarta la que obliga a achicar.
                let n_opciones = PISOS.len() + 1;
                let pw = (ANCHO as f32 * PLACA_ANCHO)
                    .min(ANCHO as f32 * FILA_UTIL / n_opciones as f32);
                let ph = pw / PLACA_RATIO;
                let hueco_p = (ANCHO as f32 - pw * n_opciones as f32) / (n_opciones + 1) as f32;
                let py0 = ALTO as f32 * PLACAS_Y;
                let pulso = 0.85 + 0.15 * (tiempo * 3.2).sin();
                let tam_placa = (ALTO as f32 * TAM_PLACA) as i32;

                for i in 0..n_opciones {
                    let placa = Rectangle::new(hueco_p + (pw + hueco_p) * i as f32, py0, pw, ph);
                    let elegida = i == piso;
                    // el infinito tiene su propio png de una sola fila; los
                    // pisos van por su fila dentro de placas.png
                    let (hoja, fila_hoja) = if i == INFINITO {
                        (&placa_inf, 0.0)
                    } else {
                        (&placas, i as f32)
                    };
                    // por get() y no por PISOS[i]: la cuarta opcion no tiene
                    // ConfigPiso, y quedarse corto en la fila es justo el caso
                    // del infinito, no un error
                    let nombre = PISOS.get(i).map_or("INFINITO", |c| c.nombre);

                    match hoja {
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
                                Rectangle::new(col * PLACA_W, fila_hoja * PLACA_H, PLACA_W, PLACA_H),
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

                            let (tx, ty, tt) = ajustar_centrado(&dh, &fuentes, nombre,
                                placa.x + placa.width / 2.0, placa.y + placa.height / 2.0,
                                tam_placa, placa.width * 0.8);
                            dibujar_texto(&mut dh, &fuentes, nombre, tx, ty, tt, tinta);
                        }
                    }
                }

                // ---------------------------------------- suciedad
                vineta(&mut dh, VINETA_BIENVENIDA.0, VINETA_BIENVENIDA.1);

                // ---------------------------------------- letra chica
                // Va DESPUES de la vineta: al bajar la marquesina estas dos
                // lineas quedaron dentro de la banda oscura de abajo y no se
                // leian. Son HUD, no escena, asi que no las tapa.
                // el infinito todavia no existe como Infinito cuando esta nada
                // mas elegido, asi que la letra chica muestra con que arranca:
                // los mismos numeros que le pone Infinito::primero()
                let chico = (ALTO as f32 * TAM_CHICO) as i32;
                let linea = if piso == INFINITO {
                    format!(
                        "cuota {}  -  {} giros  -  cada piso: +{} de cuota y un laberinto mas grande",
                        PISOS[0].cuota, PISOS[0].giros, INF_CUOTA_EXTRA
                    )
                } else {
                    let cfg = PISOS[piso];
                    format!("cuota {}  -  {} giros", cfg.cuota, cfg.giros)
                };
                texto_centrado(&mut dh, &fuentes, &linea,
                    cx, (ALTO as f32 * INFO_Y) as i32, chico, LATON);
                texto_centrado(&mut dh, &fuentes, "A D  o  flechas   elegir piso        ENTER  entrar",
                    cx, (ALTO as f32 * CONTROLES_Y) as i32, chico, LATON_TENUE);

                grano(&mut dh);
                efecto_vhs(&mut dh, tiempo, ALTO);
                fundido.velo(&mut dh);
            }

            // ============================================ MAQUINA
            Escena::Maquina => {
                // durante el fundido de salida no se relanza: si no, la pausa en
                // negro al fallar la cuota sonaria con la musica del casino
                if !fundido.en_curso() {
                    audio.maquina();
                }

                // se mira la fase antes y despues para saber si en este frame
                // freno un rodillo o si recien apareceio el resultado
                let fase_antes = anim.fase;
                let termino_anim = anim.actualizar(dt);
                if rodillos_parados(anim.fase) > rodillos_parados(fase_antes) {
                    audio.rodillo_para();
                }
                if anim.mostrando_resultado()
                    && !matches!(fase_antes, FaseRodillos::Resultado(_))
                    && anim.gano_giro
                {
                    audio.pago();
                }

                // El contador se entera del pago recien cuando los rodillos
                // revelan. Mientras giran se queda en lo que habia antes.
                if !anim.activa() || anim.mostrando_resultado() {
                    cred_objetivo = maq.creditos;
                }
                // y de ahi sube contando, no salta: el numero subiendo es lo que
                // se lee como "ganaste", un salto seco no se nota
                let objetivo = cred_objetivo as f32;
                if (cred_vista - objetivo).abs() < 0.5 {
                    cred_vista = objetivo;
                } else {
                    cred_vista += (objetivo - cred_vista) / T_CONTEO * dt;
                }

                // La sombra NO avanza mientras se juega a la maquina. Antes si, con
                // la idea de que cada giro costara tiempo real, pero en la maquina el
                // jugador no se puede mover: si la sombra venia cerca, era muerte
                // segura y sin nada que hacer al respecto. Ahora se congela donde
                // estaba y retoma desde ahi al salir.
                //
                // Los relojes SI siguen: correr_relojes() adelanta t_espera y anim_t.
                // Si se saltara perseguir() a secas, una sombra que todavia no salio
                // no se despertaria nunca mientras jugas, y el jugador podria quedarse
                // en la maquina esperando a que se le pase el peligro.
                if maquina_en_laberinto {
                    est.correr_relojes(dt);
                    if est.atrapado {
                        frase_muerte = (est.anim_t * 1000.0) as usize % 5;
                        fundido.ir_a(Escena::Derrota);
                    }
                }

                // jalar la palanca: el RNG se resuelve aca, los rodillos lo revelan despues
                // Jalar: la palanca baja PRIMERO. La guarda suma !palanca.activa()
                // porque durante la bajada la animacion todavia no arranco y sin
                // eso se colaba un segundo jalon en esos 0.10s.
                if !anim.activa() && !palanca.activa() && !fundido.en_curso() && !maq.termino
                    && (rl.is_key_pressed(KeyboardKey::KEY_F)
                        || boton(&rl, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN)) {
                    palanca.jalar();
                    audio.palanca(); // el clunk va aca, al apretar
                }
                // el giro se resuelve recien cuando la palanca toca el fondo
                if palanca.actualizar(dt) && !maq.termino {
                    let antes = maq.creditos;
                    maq.girar();
                    anim.iniciar(maq.rodillos, maq.creditos - antes);
                    // aca y en ningun otro lado: palanca.actualizar() devuelve
                    // true un solo frame, el del arranque del giro
                    audio.slot();
                }

                // salir por voluntad propia
                if !anim.activa() && !palanca.activa() && !fundido.en_curso()
                    && (rl.is_key_pressed(KeyboardKey::KEY_ENTER)
                        || boton(&rl, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT)) {
                    if !maquina_en_laberinto {
                        // se va por su cuenta: se lleva la ventaja
                        est = match &inf {
                            Some(i) => entrar_generado(i, true),
                            None => entrar_al_laberinto(&PISOS[piso], true),
                        };
                        salio_voluntario = true;
                    }
                    fundido.ir_a(Escena::Jugando);
                }

                // recien cuando la animacion termino de mostrar el resultado se decide
                if termino_anim && maq.termino {
                    if maq.gano() {
                        if maquina_en_laberinto {
                            // pago desde adentro: la maquina no es la puerta de
                            // salida. Se apaga la sombra y se vuelve al pasillo,
                            // el final se cobra recien al pisar la 'B'.
                            est.persiguiendo = false;
                            // sin esto, una espera a medio correr la despierta de
                            // nuevo apenas llega a cero
                            est.t_espera = 0.0;
                            pago_cuota = true;
                            fundido.ir_a(Escena::Jugando);
                        } else if inf.is_some() {
                            // en infinito pagar la cuota no cierra la partida:
                            // ese piso quedo saldado y se pasa al siguiente
                            avanzar_infinito = true;
                            fundido.ir_a(Escena::Maquina);
                        } else {
                            // pego la cuota sin haber entrado: se cierra en verde
                            fundido.ir_a(Escena::Exito);
                        }
                    } else {
                        // se le acabaron los giros sin llegar a la cuota
                        audio.fallo();
                        if !maquina_en_laberinto {
                            // lo tiran adentro: sin ventaja, la sombra ya salio
                            est = match &inf {
                                Some(i) => entrar_generado(i, false),
                                None => entrar_al_laberinto(&PISOS[piso], false),
                            };
                            salio_voluntario = false;
                            // corte seco de la musica y un rato en negro antes
                            // del laberinto: se acabo la fiesta
                            audio.silencio();
                            fundido.ir_a_con_pausa(Escena::Jugando, T_PAUSA_FALLO);
                        }
                        // adentro del laberinto se queda hasta que salga con ENTER
                    }
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

                // La palanca va ANTES del gabinete a proposito: asi la base
                // metalica le tapa la varilla cuando baja y se ve como que se
                // mete adentro. Dibujada despues quedaria flotando encima.
                if let (Some(tex), true) = (&palanca_tex, hay_arte) {
                    let pal_y = gy + lado * (PALANCA_Y + PALANCA_RECORRIDO * palanca.t);
                    dh.draw_texture_pro(
                        tex,
                        Rectangle::new(0.0, 0.0, PALANCA_PX.0, PALANCA_PX.1),
                        Rectangle::new(
                            gx + lado * PALANCA_X, pal_y,
                            lado * PALANCA_W, lado * PALANCA_H,
                        ),
                        Vector2::zero(), 0.0, Color::WHITE,
                    );
                }

                // rect completo de cada ventana, y tamano del simbolo de texto
                let (slots, tam_simbolo): ([Rectangle; 3], i32) = match &gabinete {
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

                        let mut sl = [Rectangle::new(0.0, 0.0, 0.0, 0.0); 3];
                        for (e, c) in sl.iter_mut().zip(CENTROS_SLOT.iter()) {
                            *e = Rectangle::new(
                                gx + lado * (c - ANCHO_SLOT / 2.0),
                                gy + lado * Y_SLOT,
                                lado * ANCHO_SLOT,
                                lado * ALTO_SLOT,
                            );
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
                        let mut sl = [Rectangle::new(0.0, 0.0, 0.0, 0.0); 3];
                        for (i, e) in sl.iter_mut().enumerate() {
                            let rx = rx0 + i as i32 * (RW + GAP);
                            *e = Rectangle::new(rx as f32, ry as f32, RW as f32, RH as f32);
                            dh.draw_rectangle(rx, ry, RW, RH, RODILLO_BG);
                            dh.draw_rectangle_lines_ex(*e, 2.0, NEON);
                        }
                        (sl, 120)
                    }
                };

                // ---- simbolos: la tira scrolleando adentro de cada ventana
                let finales = anim.finales();
                for (i, slot) in slots.iter().enumerate() {
                    let cx_slot = slot.x + slot.width / 2.0;
                    let cy_slot = slot.y + slot.height / 2.0;
                    // ganador solo cuando ya paro todo y el giro pago
                    let s = finales[i];
                    let ganador = mostrar_res && gano_giro
                        && finales.iter().filter(|o| **o == s).count() >= 2;

                    match &simbolos_tex {
                        Some(tex) => {
                            // el paso de la tira es el ancho de la ventana: las
                            // celdas son cuadradas y asi no quedan huecos
                            let paso = slot.width;
                            let off = anim.offset_rodillo(i);
                            let base = off.floor() as i32;
                            let frac = off - off.floor();
                            // los que no entraron en la combinacion van apagados
                            let tinte = if mostrar_res && gano_giro && !ganador {
                                atenuar(Color::WHITE, 0.55)
                            } else {
                                Color::WHITE
                            };

                            // SIN esto los simbolos de arriba y de abajo se
                            // salen de la ventana y se dibujan sobre el gabinete
                            let mut sc = dh.begin_scissor_mode(
                                slot.x as i32, slot.y as i32,
                                slot.width as i32, slot.height as i32,
                            );
                            for k in -1..=1 {
                                let idx = (base - k).rem_euclid(N_SIMBOLOS as i32) as f32;
                                let y = cy_slot + (frac + k as f32) * paso - paso / 2.0;
                                sc.draw_texture_pro(
                                    tex,
                                    Rectangle::new(0.0, idx * CELDA_SIMBOLO, CELDA_SIMBOLO, CELDA_SIMBOLO),
                                    Rectangle::new(cx_slot - paso / 2.0, y, paso, paso),
                                    Vector2::zero(), 0.0, tinte,
                                );
                            }
                        }
                        None => {
                            // fallback: la letra del simbolo centrado, que sigue
                            // el mismo offset asi acompana el scroll
                            let v = anim.simbolo_centrado(i);
                            let bob = if anim.activa() { 0.0 } else { flota(tiempo + i as f32, 2.0, 3.0) as f32 };
                            let (tx, ty, tt) = ajustar_centrado(
                                &dh, &fuentes, v.letra(), cx_slot, cy_slot + bob,
                                tam_simbolo, slot.width,
                            );
                            if ganador {
                                texto_glow(&mut dh, &fuentes, v.letra(), tx, ty, tt, color_simbolo(v, true));
                            } else {
                                dibujar_texto(&mut dh, &fuentes, v.letra(), tx, ty, tt, color_simbolo(v, false));
                            }
                        }
                    }
                }

                // fondo negro con vineta: nada compite con el gabinete
                if hay_arte {
                    vineta(&mut dh, VINETA_MAQUINA.0, VINETA_MAQUINA.1);
                }

                // ---- HUD, siempre afuera del gabinete
                let rotulo = match &inf {
                    Some(i) => format!("PISO {}", i.n),
                    None => format!("RONDA {} DE {}", piso + 1, PISOS.len()),
                };
                dibujar_texto(&mut dh, &fuentes, &rotulo,
                    MARGEN_HUD, MARGEN_HUD, 26, TEXTO);
                // mientras el contador sube se pinta del color del pago
                let contando = (cred_vista - cred_objetivo as f32).abs() >= 0.5;
                let col_cred = if contando { anim.resultado_color } else { TEXTO };
                texto_derecha(&mut dh, &fuentes,
                    &format!("CREDITOS {} / {}", cred_vista.round() as i32, maq.cuota),
                    ANCHO - MARGEN_HUD, MARGEN_HUD, 26, col_cred);
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
                    let salida = if maquina_en_laberinto {
                        "ENTER volver"
                    } else {
                        "ENTER salir ya, con ventaja"
                    };
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
                    let alerta = alerta_de(&inf, piso);
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
                audio.laberinto();

                // input
                if rl.is_key_down(KeyboardKey::KEY_LEFT) {
                    est.a -= VEL_GIRO * dt;
                }

                if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
                    est.a += VEL_GIRO * dt;
                }

                let mouse_dx = rl.get_mouse_delta().x;
                est.a += mouse_dx * SENS_MOUSE;

                // stick derecho: gira igual que las flechas, pero escalado por
                // cuanto se empuja. Se suma, asi que mouse y stick conviven.
                est.a += eje(&rl, GamepadAxis::GAMEPAD_AXIS_RIGHT_X) * VEL_GIRO * dt;

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

                // stick izquierdo: lo mismo que WSQE pero analogico, la
                // velocidad sale de cuanto se empuja. Va aparte de los bloques
                // de teclado y no en un else, para que las dos entradas anden a
                // la vez en vez de que una anule a la otra.
                let stick_y = eje(&rl, GamepadAxis::GAMEPAD_AXIS_LEFT_Y);
                if stick_y != 0.0 {
                    // el eje Y del stick da NEGATIVO hacia adelante, por eso el menos
                    let p = -stick_y * paso;
                    est.avanzar(est.a.cos() * p, est.a.sin() * p);
                }
                let stick_x = eje(&rl, GamepadAxis::GAMEPAD_AXIS_LEFT_X);
                if stick_x != 0.0 {
                    let p = stick_x * paso;
                    est.avanzar(lado.cos() * p, lado.sin() * p);
                }

                est.perseguir(dt);
                // la celda que se pisa se revela siempre, incluso quieto y
                // aunque se juegue en 2D, donde no hay rayos que revelen
                est.revelar_jugador();

                // intensidad segun que tan encima esta la sombra. Sale de lo que
                // Estado ya expone: no hace falta meterle audio adentro.
                let cerca = (1.0 - est.dist_enemigo() / alerta_de(&inf, piso)).clamp(0.0, 1.0);
                audio.tension(dt, est.persiguiendo, cerca);

                if rl.is_key_pressed(KeyboardKey::KEY_M) {
                    est.modo3d = !est.modo3d;
                }
                if rl.is_key_pressed(KeyboardKey::KEY_T) {
                    usar_tex = !usar_tex;
                }

                // pared M: se entra a la maquina sin perder la posicion en el laberinto
                let junto_a_maquina = hay_adyacente(&est.grid, est.x, est.y, 'M');
                if junto_a_maquina && !maq.termino && !fundido.en_curso()
                    && (rl.is_key_pressed(KeyboardKey::KEY_F)
                        || boton(&rl, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN)) {
                    maquina_en_laberinto = true;
                    fundido.ir_a(Escena::Maquina);
                }

                // transiciones
                if est.gano {
                    // llego a la salida: cierra la partida. Con la cuota ya pagada
                    // el final es el de pagar, no el de haberse escapado corriendo;
                    // el resto lo bifurca sola la Victoria segun salio_voluntario.
                    if inf.is_some() {
                        // en infinito escaparse no es el final: es el pasaje al
                        // piso que sigue, y arranca de nuevo por la maquina
                        avanzar_infinito = true;
                        fundido.ir_a(Escena::Maquina);
                    } else if pago_cuota {
                        fundido.ir_a(Escena::Exito);
                    } else {
                        fundido.ir_a(Escena::Victoria);
                    }
                }
                if est.atrapado {
                    frase_muerte = (est.anim_t * 1000.0) as usize % 5;
                    // por fundido, igual que el resto de las transiciones
                    fundido.ir_a(Escena::Derrota);
                }

                // dibujo
                let fps = rl.get_fps();
                let dist_e = est.dist_enemigo();
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(BG);

                if est.modo3d {
                  let zbuffer = render_3d(&mut dh, &mut est, &texturas, tex_piso.as_ref(), tex_techo.as_ref(), usar_tex);
                  if est.persiguiendo {
                        render::render_sombra(&mut dh, &est, &zbuffer);
                  }
                    // el post va sobre la escena pero ANTES del minimapa: el
                    // minimapa vive en la esquina, que es justo donde las dos
                    // bandas de la vineta se pisan y el negro va doble. Detras
                    // del post no se leeria.
                    //
                    // t = est.anim_t, que es el reloj que ya usa esta escena
                    // para lo demas (el flicker del apagon, el pulso de la F).
                    // Avanza siempre, incluso con la sombra apagada.
                    render::render_post(&mut dh, est.anim_t, grano_tex.as_ref());
                    render_minimapa(&mut dh, &est);
                } else {
                    render_2d(&mut dh, &est);
                }

                // el apagon solo aplica cuando el sujeto anda suelto
                let alerta = alerta_de(&inf, piso);
                if est.persiguiendo && dist_e < alerta {
    let t = (1.0 - dist_e / alerta).clamp(0.0, 1.0);
    // el centro nunca se cierra del todo: siempre te queda por donde mirar
    let radio = APAGON_CENTRO + (1.0 - t) * (0.5 - APAGON_CENTRO * 0.5);
    for band in 0..20 {
        let bt = band as f32 / 20.0;
        if bt > radio {
            let ba = ((bt - radio) / (1.0 - radio)).clamp(0.0, 1.0);
            let a = (ba * ba * APAGON_BANDAS) as u8;
            let h = (VIEW_H as f32 * 0.05) as i32 + 1;
            dh.draw_rectangle(0, band * h, ANCHO, h, Color { r: 0, g: 0, b: 0, a });
            dh.draw_rectangle(0, VIEW_H - (band + 1) * h, ANCHO, h, Color { r: 0, g: 0, b: 0, a });
            let w = (ANCHO as f32 * 0.05) as i32 + 1;
            dh.draw_rectangle(band * w, 0, w, VIEW_H, Color { r: 0, g: 0, b: 0, a });
            dh.draw_rectangle(ANCHO - (band + 1) * w, 0, w, VIEW_H, Color { r: 0, g: 0, b: 0, a });
        }
    } // <-- cierra el for

    // esto va AFUERA del for
    let black_a = (t * t * t * APAGON_VELO) as u8;
    dh.draw_rectangle(0, 0, ANCHO, VIEW_H, Color { r: 0, g: 0, b: 0, a: black_a });

    if t > 0.6 {
        let flicker = ((est.anim_t * 12.0).sin() * (est.anim_t * 37.0).sin()).abs();
        let fa = (flicker * t * APAGON_FLICKER) as u8;
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
                // con la cuota pagada ya no hay nadie atras: solo queda irse
                let objetivo = if pago_cuota {
                    "pagaste  -  ya nadie te sigue, sali por la B"
                } else {
                    "llega a la salida"
                };
                // en infinito no hay total que mostrar: el contador sube y ya
                let ronda = match &inf {
                    Some(i) => format!("piso {}", i.n),
                    None => format!("ronda {}/{}", piso + 1, PISOS.len()),
                };
                dibujar_texto(&mut dh, &fuentes,
                    &format!("[{}]  {}  -  {}", modo, ronda, objetivo),
                    12, VIEW_H + 8, 24, TEXTO,
                );
                // la distancia al sujeto solo cuando de verdad anda suelto
                // durante la ventaja se avisa, si no el jugador no entiende
                // por que a veces la sombra no aparece
                let info = if est.t_espera > 0.0 {
                    format!("todavia no te vieron: {:.0}   {} fps", est.t_espera.ceil(), fps)
                } else if est.persiguiendo {
                    format!("el sujeto a {:.1}   {} fps", dist_e, fps)
                } else {
                    format!("{} fps", fps)
                };
                dibujar_texto(&mut dh, &fuentes,
                    &info, ANCHO - 260, VIEW_H + 8, 24,
                    if est.t_espera > 0.0 { CYAN }
                    else if est.persiguiendo && dist_e < alerta { ENEMIGO }
                    else { Color { r: 140, g: 110, b: 180, a: 255 } },
                );
                fundido.velo(&mut dh);
            }



    



            // ============================================ EXITO
            // Pegar la cuota es la unica salida limpia: no corriste, pagaste.
            Escena::Exito => {
                audio.silencio();
                if rl.is_key_pressed(KeyboardKey::KEY_R) || boton_cualquiera(&rl) {
                    escena = Escena::Bienvenida;
                }

                // Se centra sobre ALTO, no sobre VIEW_H: aca no hay franja de
                // HUD, VIEW_H dejaba todo 20px arriba del centro real.
                let cx = ANCHO / 2;
                let cy = ALTO / 2;
                let nombre = nombre_de(&inf, piso);
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(Color { r: 0, g: 0, b: 0, a: 255 });

                texto_vhs(&mut dh, &fuentes, "CUMPLISTE LA CUOTA", cx,
                    cy - 96 + flota(tiempo, 1.4, 5.0), 52, DORADO, tiempo);
                texto_centrado(&mut dh, &fuentes,
                    &format!("{}  -  {} de {} creditos", nombre, maq.creditos, maq.cuota),
                    cx, cy - 12, 30, TEXTO);
                texto_centrado(&mut dh, &fuentes, "te vas caminando, no corriendo",
                    cx, cy + 36, 26, CYAN);
                texto_glow_centrado(&mut dh, &fuentes, "R   volver al menu", cx,
                    cy + 116 + flota(tiempo, 2.4, 2.0), 28, NEON);

                vineta(&mut dh, VINETA_BIENVENIDA.0, VINETA_BIENVENIDA.1);
                grano(&mut dh);
                efecto_vhs(&mut dh, tiempo, ALTO);
                fundido.velo(&mut dh);
            }

            // ============================================ VICTORIA
            Escena::Victoria => {
                audio.silencio();
                if rl.is_key_pressed(KeyboardKey::KEY_R) || boton_cualquiera(&rl) {
                    // se conserva el piso elegido, para reintentarlo directo
                    escena = Escena::Bienvenida;
                }

                // Se centra sobre ALTO, no sobre VIEW_H: aca no hay franja de
                // HUD, VIEW_H dejaba todo 20px arriba del centro real.
                let cx = ANCHO / 2;
                let cy = ALTO / 2;
                let nombre = nombre_de(&inf, piso);
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(Color { r: 0, g: 0, b: 0, a: 255 });

                if salio_voluntario {
                    // Ruta fria: te fuiste antes de que te obligaran. No hay logro que
                    // cantar, asi que no va ni NEON ni CYAN: sale todo en laton apagado.
                    texto_centrado(&mut dh, &fuentes, "TE FUISTE", cx,
                        cy - 96 + flota(tiempo, 0.8, 2.0), 58, LATON);
                    texto_centrado(&mut dh, &fuentes,
                        &format!("{}  -  {} de {} creditos", nombre, maq.creditos, maq.cuota),
                        cx, cy - 12, 30, LATON_TENUE);
                    texto_centrado(&mut dh, &fuentes, "te vas con lo que trajiste",
                        cx, cy + 36, 26, LATON_TENUE);
                    texto_centrado(&mut dh, &fuentes, "R   volver al menu", cx,
                        cy + 116, 28, LATON_TENUE);
                } else {
                    texto_vhs(&mut dh, &fuentes, "ESCAPASTE", cx,
                        cy - 96 + flota(tiempo, 1.6, 5.0), 58, NEON, tiempo);
                    texto_centrado(&mut dh, &fuentes,
                        &format!("{}  -  llegaste a la salida", nombre),
                        cx, cy - 12, 30, TEXTO);
                    texto_centrado(&mut dh, &fuentes, "saliste corriendo, pero saliste",
                        cx, cy + 36, 26, CYAN);
                    texto_glow_centrado(&mut dh, &fuentes, "R   volver al menu", cx,
                        cy + 116 + flota(tiempo, 2.4, 2.0), 28, NEON);
                }

                vineta(&mut dh, VINETA_BIENVENIDA.0, VINETA_BIENVENIDA.1);
                grano(&mut dh);
                efecto_vhs(&mut dh, tiempo, ALTO);
                fundido.velo(&mut dh);
            }

            // ============================================ DERROTA
            Escena::Derrota => {
                audio.silencio();

                const FRASES: [&str; 5] = [
                    "La ambicion mata",
                    "La casa siempre gana",
                    "Debiste parar cuando podias",
                    "Nadie sale de aqui",
                    "El juego estaba arreglado",
                ];

                if rl.is_key_pressed(KeyboardKey::KEY_R) || boton_cualquiera(&rl) {
                    // se conserva el piso elegido, para reintentarlo directo
                    escena = Escena::Bienvenida;
                }

                let cx = ANCHO / 2;
                let cy = ALTO / 2;
                let mut dh = rl.begin_drawing(&thread);
                dh.clear_background(Color { r: 2, g: 0, b: 0, a: 255 });

                texto_vhs(&mut dh, &fuentes, FRASES[frase_muerte], cx,
                    cy - 60 + flota(tiempo, 1.1, 4.0), 50, META, tiempo * 2.0);
                texto_centrado(&mut dh, &fuentes, "te alcanzo en el laberinto",
                    cx, cy + 20, 26, Color { r: 150, g: 110, b: 115, a: 255 });
                texto_glow_centrado(&mut dh, &fuentes, "R   volver al menu", cx,
                    cy + 116 + flota(tiempo, 2.4, 2.0), 28, META);

                vineta(&mut dh, VINETA_BIENVENIDA.0, VINETA_BIENVENIDA.1);
                grano(&mut dh);
                efecto_vhs(&mut dh, tiempo, ALTO);
                fundido.velo(&mut dh);
            }
        }

    }
}
