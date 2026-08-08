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

impl Simbolo {
    /// letra que se dibuja en el rodillo (la fuente de raylib no tiene emoji)
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

const T_GIRO: f32 = 0.8;      // los 3 rodillos sueltos antes de la primera parada
const T_PARADA: f32 = 0.3;    // espera entre parada y parada
const T_RESULTADO: f32 = 1.5; // cuanto queda el resultado en pantalla
const T_CAMBIO: f32 = 0.05;   // cada cuanto cambia el simbolo de un rodillo suelto

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
    pub simbolos_visual: [Simbolo; 3],
    pub resultado_texto: String,
    pub resultado_color: Color,
    pub gano_giro: bool, // el giro pago algo
    pub total: f32, // reloj global, para los parpadeos
    finales: [Simbolo; 3],
    t_cambio: f32,
}

impl AnimRodillos {
    pub fn nueva() -> Self {
        AnimRodillos {
            fase: FaseRodillos::Idle,
            simbolos_visual: [Simbolo::Cereza, Simbolo::Siete, Simbolo::Diamante],
            resultado_texto: String::new(),
            resultado_color: ROJO,
            gano_giro: false,
            total: 0.0,
            finales: [Simbolo::Cereza; 3],
            t_cambio: 0.0,
        }
    }

    pub fn iniciar(&mut self, finales: [Simbolo; 3], pago: i32) {
        self.fase = FaseRodillos::Girando(0.0);
        self.finales = finales;
        self.t_cambio = 0.0;
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

        // los rodillos que todavia giran cambian de simbolo rapido
        self.t_cambio += dt;
        if self.t_cambio >= T_CAMBIO {
            self.t_cambio = 0.0;
            let parados = self.parados();
            for i in parados..3 {
                let k = rand::thread_rng().gen_range(0..SIMBOLOS.len());
                self.simbolos_visual[i] = SIMBOLOS[k];
            }
        }

        match self.fase {
            FaseRodillos::Idle => {}
            FaseRodillos::Girando(t) => {
                let t = t + dt;
                if t >= T_GIRO {
                    self.simbolos_visual[0] = self.finales[0];
                    self.fase = FaseRodillos::Parando(0, 0.0);
                } else {
                    self.fase = FaseRodillos::Girando(t);
                }
            }
            FaseRodillos::Parando(i, t) => {
                let t = t + dt;
                if t >= T_PARADA {
                    if i + 1 < 3 {
                        self.simbolos_visual[i + 1] = self.finales[i + 1];
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
                    return true;
                }
                self.fase = FaseRodillos::Resultado(t);
            }
        }
        false
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
        self.simbolos_visual == [Simbolo::Siete; 3]
    }

    pub fn maldicion(&self) -> bool {
        self.simbolos_visual == [Simbolo::Calavera; 3]
    }
}
