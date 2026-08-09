// maquina.rs — tragamonedas: rodillos, pagos, cuota, animacion de los rodillos

use rand::Rng;
use raylib::prelude::Color;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Simbolo {
    Cereza,
    Campana,
    Diamante,
    Siete,
    Calavera,
}

const SIMBOLOS: [Simbolo; 5] = [
    Simbolo::Cereza,
    Simbolo::Campana,
    Simbolo::Diamante,
    Simbolo::Siete,
    Simbolo::Calavera,
];

/// cuantos simbolos tiene la tira. La tira del spritesheet va en este mismo
/// orden, asi que indice() sirve de indice de celda y de posicion en la tira.
pub const N_SIMBOLOS: usize = SIMBOLOS.len();

impl Simbolo {
    /// indice de celda en assets/sprites/simbolos.png, y posicion en la tira
    pub fn indice(&self) -> usize {
        match self {
            Simbolo::Cereza => 0,
            Simbolo::Campana => 1,
            Simbolo::Diamante => 2,
            Simbolo::Siete => 3,
            Simbolo::Calavera => 4,
        }
    }

    /// letra que se dibuja en el rodillo. Queda como fallback para cuando no
    /// esta la textura de simbolos.
    pub fn letra(&self) -> &'static str {
        match self {
            Simbolo::Cereza => "C",
            Simbolo::Campana => "B",
            Simbolo::Diamante => "D",
            Simbolo::Siete => "7",
            Simbolo::Calavera => "X",
        }
    }
}

pub struct Maquina {
    pub rodillos: [Simbolo; 3],
    pub creditos: i32,
    pub cuota: i32,
    pub giros_restantes: i32,
    pub termino: bool,
}

impl Maquina {
    pub fn nueva(cuota: i32, giros: i32) -> Self {
        Maquina {
            rodillos: [Simbolo::Cereza; 3],
            creditos: 0,
            cuota,
            giros_restantes: giros,
            termino: false,
        }
    }

    pub fn girar(&mut self) {
        if self.termino || self.giros_restantes <= 0 {
            return;
        }
        for r in self.rodillos.iter_mut() {
           let i = rand::thread_rng().gen_range(0..SIMBOLOS.len());
            *r = SIMBOLOS[i];
        }
        self.creditos += self.pago();
        self.giros_restantes -= 1;

        if self.creditos >= self.cuota || self.giros_restantes <= 0 {
            self.termino = true;
        }
    }

    pub fn gano(&self) -> bool {
        self.termino && self.creditos >= self.cuota
    }

    fn pago(&self) -> i32 {
        let [a, b, c] = self.rodillos;
        if a == b && b == c {
            // triples
            match a {
                Simbolo::Siete => 50,
                Simbolo::Diamante => 30,
                Simbolo::Campana => 20,
                Simbolo::Cereza => 15,
                Simbolo::Calavera => -10,
            }
        } else if a == b || b == c || a == c {
            // par
            5
        } else {
            0
        }
    }
}

// ==================================================== animacion de los rodillos

// Del jalon al "+30" pasan T_GIRO + 3*T_PARADA, y hasta poder volver a jalar
// eso mas T_RESULTADO. Con 0.8/0.3/1.5 eran 1.7s hasta el pago y 3.2s de ciclo:
// por 20 giros son mas de un minuto mirando rodillos. Bajado a 1.04s y 2.14s,
// que sigue leyendose como tragamonedas pero no se hace esperar.
const T_GIRO: f32 = 0.5;      // los 3 rodillos sueltos antes de la primera parada
const T_PARADA: f32 = 0.18;   // espera entre parada y parada
const T_RESULTADO: f32 = 1.1; // cuanto queda el resultado en pantalla

// ---- scroll de la tira de simbolos
const VEL_GIRO: f32 = 18.0; // simbolos por segundo al soltar
const VEL_MIN: f32 = 4.0;   // piso al que baja la velocidad antes del snap
// Con 28 el rodillo 0 llega justo a VEL_MIN en sus 0.5s de giro; los otros dos
// llegan antes de su turno y se arrastran lento hasta encajar.
const FRENADO: f32 = 28.0;  // cuanto baja la velocidad por segundo
const REBOTE: f32 = 0.12;   // cuanto se pasa al parar, en unidades de simbolo
const T_REBOTE: f32 = 0.12;

// ---- palanca
const T_PALANCA_BAJA: f32 = 0.10; // la bajada es seca
const T_PALANCA_SUBE: f32 = 0.28; // la subida vuelve mas lenta, a proposito

const VERDE: Color = Color { r: 90, g: 230, b: 120, a: 255 };
const ROJO: Color = Color { r: 204, g: 34, b: 34, a: 255 };

#[derive(Clone, Copy, PartialEq)]
pub enum FaseRodillos {
    Idle,
    Girando(f32),
    Parando(usize, f32),
    Resultado(f32),
}

/// El RNG ya lo resolvio Maquina::girar(); esto nomas revela ese resultado
/// rodillo por rodillo y se queda un rato mostrando el pago.
pub struct AnimRodillos {
    pub fase: FaseRodillos,
    pub resultado_texto: String,
    pub resultado_color: Color,
    pub gano_giro: bool, // el giro pago algo
    pub total: f32, // reloj global, para los parpadeos
    finales: [Simbolo; 3],
    /// posicion de cada rodillo en la tira, en unidades de simbolo. La parte
    /// entera es que simbolo esta centrado, la fraccionaria el desplazamiento.
    offset: [f32; 3],
    vel: [f32; 3],
    /// lo que queda del rebote de cada rodillo despues del snap
    t_rebote: [f32; 3],
}

impl AnimRodillos {
    pub fn nueva() -> Self {
        AnimRodillos {
            fase: FaseRodillos::Idle,
            resultado_texto: String::new(),
            resultado_color: ROJO,
            gano_giro: false,
            total: 0.0,
            finales: [Simbolo::Cereza; 3],
            // arranca mostrando tres simbolos distintos, no tres cerezas
            offset: [0.0, 3.0, 2.0],
            vel: [0.0; 3],
            t_rebote: [0.0; 3],
        }
    }

    /// posicion de la tira del rodillo i, para dibujarla
    pub fn offset_rodillo(&self, i: usize) -> f32 {
        self.offset[i]
    }

    /// los simbolos que salieron. Es la fuente de verdad del resultado: los
    /// rodillos solo lo revelan.
    pub fn finales(&self) -> [Simbolo; 3] {
        self.finales
    }

    /// simbolo centrado en la ventana del rodillo i, sacado del offset. Lo usa
    /// el fallback de texto para acompanar el scroll.
    pub fn simbolo_centrado(&self, i: usize) -> Simbolo {
        let pos = self.offset[i].rem_euclid(N_SIMBOLOS as f32);
        SIMBOLOS[(pos.floor() as usize).min(N_SIMBOLOS - 1)]
    }

    pub fn iniciar(&mut self, finales: [Simbolo; 3], pago: i32) {
        self.fase = FaseRodillos::Girando(0.0);
        self.finales = finales;
        self.vel = [VEL_GIRO; 3];
        self.t_rebote = [0.0; 3];
        let (txt, col) = if pago > 0 {
            (format!("+{}", pago), VERDE)
        } else if pago < 0 {
            (format!("{}", pago), ROJO)
        } else {
            ("NADA".to_string(), ROJO)
        };
        self.resultado_texto = txt;
        self.resultado_color = col;
        self.gano_giro = pago > 0;
    }

    /// devuelve true en el frame en que la animacion termina de mostrar el resultado
    pub fn actualizar(&mut self, dt: f32) -> bool {
        self.total += dt;

        // cuantos rodillos estaban quietos antes de correr la maquina de fases
        let parados_antes = self.parados();
        let mut termino = false;

        // ---- fases: tal cual estaban, no se les toco ni la logica ni los tiempos
        match self.fase {
            FaseRodillos::Idle => {}
            FaseRodillos::Girando(t) => {
                let t = t + dt;
                if t >= T_GIRO {
                    self.fase = FaseRodillos::Parando(0, 0.0);
                } else {
                    self.fase = FaseRodillos::Girando(t);
                }
            }
            FaseRodillos::Parando(i, t) => {
                let t = t + dt;
                if t >= T_PARADA {
                    if i + 1 < 3 {
                        self.fase = FaseRodillos::Parando(i + 1, 0.0);
                    } else {
                        self.fase = FaseRodillos::Resultado(0.0);
                    }
                } else {
                    self.fase = FaseRodillos::Parando(i, t);
                }
            }
            FaseRodillos::Resultado(t) => {
                let t = t + dt;
                if t >= T_RESULTADO {
                    self.fase = FaseRodillos::Idle;
                    termino = true;
                }
                if !termino {
                    self.fase = FaseRodillos::Resultado(t);
                }
            }
        }

        // ---- scroll de las tiras
        let parados = self.parados();
        // los que acaban de frenar en este frame arrancan su rebote
        for i in parados_antes..parados.min(3) {
            self.t_rebote[i] = T_REBOTE;
        }

        for i in 0..3 {
            if i < parados {
                // ya paro: queda clavado en su simbolo final, con el rebote
                // corto encima mientras dure
                if self.t_rebote[i] > 0.0 {
                    self.t_rebote[i] = (self.t_rebote[i] - dt).max(0.0);
                }
                let destino = self.finales[i].indice() as f32;
                self.offset[i] = destino + REBOTE * (self.t_rebote[i] / T_REBOTE);
            } else {
                // sigue girando: la velocidad cae hacia VEL_MIN y la tira corre
                self.vel[i] = (self.vel[i] - FRENADO * dt).max(VEL_MIN);
                self.offset[i] =
                    (self.offset[i] + self.vel[i] * dt).rem_euclid(N_SIMBOLOS as f32);
            }
        }

        termino
    }

    pub fn activa(&self) -> bool {
        self.fase != FaseRodillos::Idle
    }

    /// cuantos rodillos ya se fijaron en su simbolo final
    fn parados(&self) -> usize {
        match self.fase {
            FaseRodillos::Girando(_) => 0,
            FaseRodillos::Parando(i, _) => i + 1,
            _ => 3,
        }
    }

    pub fn mostrando_resultado(&self) -> bool {
        matches!(self.fase, FaseRodillos::Resultado(_))
    }

    pub fn bonus(&self) -> bool {
        self.finales == [Simbolo::Siete; 3]
    }

    pub fn maldicion(&self) -> bool {
        self.finales == [Simbolo::Calavera; 3]
    }
}

// ==================================================== palanca

#[derive(Clone, Copy, PartialEq)]
enum FasePalanca {
    Quieta,
    Bajando(f32),
    Subiendo(f32),
}

/// Estado de la palanca. Va aparte de AnimRodillos a proposito: la palanca
/// corre ANTES de que exista el giro (es la que lo dispara), asi que no puede
/// vivir adentro de la animacion que ella misma arranca.
pub struct Palanca {
    /// 0.0 = arriba, 1.0 = abajo del todo
    pub t: f32,
    fase: FasePalanca,
}

impl Palanca {
    pub fn nueva() -> Self {
        Palanca { t: 0.0, fase: FasePalanca::Quieta }
    }

    /// arranca el jalon. No hace nada si ya hay uno en curso.
    pub fn jalar(&mut self) {
        if self.fase == FasePalanca::Quieta {
            self.fase = FasePalanca::Bajando(0.0);
        }
    }

    /// hay un jalon en curso. Cubre tambien la bajada, que es el rato en que
    /// los rodillos todavia no arrancaron y se podria colar un segundo jalon.
    pub fn activa(&self) -> bool {
        self.fase != FasePalanca::Quieta
    }

    /// Avanza. Devuelve true en el unico frame en que la palanca toca el fondo:
    /// ahi es donde el que llama resuelve el giro. Ese desfase es lo que hace
    /// sentir que la palanca causo el giro y no que pasaron juntos de casualidad.
    pub fn actualizar(&mut self, dt: f32) -> bool {
        match self.fase {
            FasePalanca::Quieta => false,
            FasePalanca::Bajando(t) => {
                let t = t + dt;
                if t >= T_PALANCA_BAJA {
                    self.t = 1.0;
                    self.fase = FasePalanca::Subiendo(0.0);
                    true // toco el fondo
                } else {
                    self.t = t / T_PALANCA_BAJA;
                    self.fase = FasePalanca::Bajando(t);
                    false
                }
            }
            FasePalanca::Subiendo(t) => {
                let t = t + dt;
                if t >= T_PALANCA_SUBE {
                    self.t = 0.0;
                    self.fase = FasePalanca::Quieta;
                } else {
                    self.t = 1.0 - t / T_PALANCA_SUBE;
                    self.fase = FasePalanca::Subiendo(t);
                }
                false
            }
        }
    }
}
