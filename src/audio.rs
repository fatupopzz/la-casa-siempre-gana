// audio.rs — musica por escena, efectos, y la tension segun que tan encima
// esta la sombra.
//
// El dispositivo (RaylibAudio) NO vive aca: Music y Sound llevan un &'aud
// RaylibAudio adentro, asi que un struct que tuviera el dispositivo y las
// pistas seria autoreferencial y no compila. El dispositivo se queda en main y
// esto lo toma prestado; por eso el lifetime en Audio<'a>.

use raylib::prelude::*;

// -------------------------------------------------- volumenes
pub const VOL_BIENVENIDA: f32 = 0.25; // la musica se escucha de lejos
pub const VOL_CASINO: f32 = 0.7;      // parado frente a la maquina
const T_SUBIDA: f32 = 0.5;            // cuanto tarda ese salto
// La velocidad se calibra contra ese salto: asi bienvenida -> maquina dura
// T_SUBIDA aunque despues se use el mismo fade para el drone.
const VEL_VOLUMEN: f32 = (VOL_CASINO - VOL_BIENVENIDA) / T_SUBIDA;

const VOL_DRONE_BASE: f32 = 0.2;  // con la sombra lejos o quieta
const VOL_DRONE_CERCA: f32 = 0.5; // lo que suma cuando esta encima

// -------------------------------------------------- latido y pasos
const LATIDO_LEJOS: f32 = 1.2;  // segundos entre golpes, sombra lejos
const LATIDO_CERCA: f32 = 0.35; // encima
const VOL_LATIDO: f32 = 0.9;
const PASOS_LEJOS: f32 = 0.85;  // los pasos van mas seguidos que el latido
const PASOS_CERCA: f32 = 0.30;
const VOL_PASOS: f32 = 0.6;

// -------------------------------------------------- rutas
// musica_casino.mp3 no esta en el repo por copyright: si falta, el juego
// arranca en silencio y sigue andando.
const RUTA_MUSICA: &str = "assets/audio/musica_casino.mp3";
const RUTA_DRONE: &str = "assets/audio/drone.wav";
const RUTA_LATIDO: &str = "assets/audio/latido.wav";
// el giro de los rodillos
const RUTA_SLOT: &str = "assets/audio/slot.wav";
// el clunk seco de la palanca, que suena al apretar, 0.10s antes que el giro
const RUTA_PALANCA: &str = "assets/audio/palanca.wav";
// estos todavia no existen: quedan en None y el juego corre igual
const RUTA_RODILLO: &str = "assets/audio/rodillo.wav";
const RUTA_PAGO: &str = "assets/audio/pago.wav";
const RUTA_FALLO: &str = "assets/audio/fallo.wav";
const RUTA_PASOS: &str = "assets/audio/pasos.wav";

/// que pista esta sonando ahora. Sirve para no reiniciar la musica del casino
/// cuando se pasa de la bienvenida a la maquina: es la misma pista, solo sube.
#[derive(Clone, Copy, PartialEq)]
enum Pista {
    Nada,
    Casino,
    Drone,
}

/// carga una pista en loop. Si el archivo no esta, avisa una sola vez por
/// stderr (esto corre al arranque) y devuelve None.
fn cargar_musica<'a>(dispositivo: &'a RaylibAudio, ruta: &str) -> Option<Music<'a>> {
    match dispositivo.new_music(ruta) {
        Ok(mut m) => {
            // el crate no expone setter de loop; el campo si, via DerefMut
            m.looping = true;
            Some(m)
        }
        Err(_) => {
            eprintln!("aviso: no encontre {}, esa pista va muda", ruta);
            None
        }
    }
}

/// carga un efecto. Mismo trato: si falta, aviso y None.
fn cargar_sonido<'a>(dispositivo: &'a RaylibAudio, ruta: &str) -> Option<Sound<'a>> {
    match dispositivo.new_sound(ruta) {
        Ok(s) => Some(s),
        Err(_) => {
            eprintln!("aviso: no encontre {}, ese efecto va mudo", ruta);
            None
        }
    }
}

/// toca el efecto si esta cargado. Si falta el archivo no pasa nada.
fn tocar(s: &Option<Sound<'_>>, volumen: f32) {
    if let Some(s) = s {
        s.set_volume(volumen);
        s.play();
    }
}

pub struct Audio<'a> {
    musica: Option<Music<'a>>,
    drone: Option<Music<'a>>,
    latido: Option<Sound<'a>>,
    slot: Option<Sound<'a>>,
    palanca: Option<Sound<'a>>,
    rodillo: Option<Sound<'a>>,
    pago: Option<Sound<'a>>,
    fallo: Option<Sound<'a>>,
    pasos: Option<Sound<'a>>,
    pista: Pista,
    vol_actual: f32,
    vol_objetivo: f32,
    t_latido: f32,
    t_pasos: f32,
}

impl<'a> Audio<'a> {
    pub fn nuevo(dispositivo: &'a RaylibAudio) -> Self {
        Audio {
            musica: cargar_musica(dispositivo, RUTA_MUSICA),
            drone: cargar_musica(dispositivo, RUTA_DRONE),
            latido: cargar_sonido(dispositivo, RUTA_LATIDO),
            slot: cargar_sonido(dispositivo, RUTA_SLOT),
            palanca: cargar_sonido(dispositivo, RUTA_PALANCA),
            rodillo: cargar_sonido(dispositivo, RUTA_RODILLO),
            pago: cargar_sonido(dispositivo, RUTA_PAGO),
            fallo: cargar_sonido(dispositivo, RUTA_FALLO),
            pasos: cargar_sonido(dispositivo, RUTA_PASOS),
            pista: Pista::Nada,
            vol_actual: 0.0,
            vol_objetivo: 0.0,
            t_latido: 0.0,
            t_pasos: 0.0,
        }
    }

    /// todo en None. Se usa cuando el dispositivo de audio no abrio, asi main
    /// no tiene que envolver cada llamada en un if.
    pub fn mudo() -> Self {
        Audio {
            musica: None,
            drone: None,
            latido: None,
            slot: None,
            palanca: None,
            rodillo: None,
            pago: None,
            fallo: None,
            pasos: None,
            pista: Pista::Nada,
            vol_actual: 0.0,
            vol_objetivo: 0.0,
            t_latido: 0.0,
            t_pasos: 0.0,
        }
    }

    // ---------------------------------------------- musica por escena

    /// musica del casino de fondo, lejos
    pub fn bienvenida(&mut self) {
        self.casino(VOL_BIENVENIDA);
    }

    /// La MISMA pista que la bienvenida, subiendo de volumen. Si ya esta
    /// sonando no se reinicia: el efecto es que te acercaste a la fuente, y se
    /// pierde si la pista vuelve a empezar.
    pub fn maquina(&mut self) {
        self.casino(VOL_CASINO);
    }

    fn casino(&mut self, objetivo: f32) {
        self.vol_objetivo = objetivo;
        if self.pista == Pista::Casino {
            return; // ya suena: solo cambia el objetivo y el fade la lleva
        }
        if let Some(d) = &self.drone {
            d.stop_stream();
        }
        if let Some(m) = &self.musica {
            m.set_volume(objetivo);
            m.play_stream();
        }
        self.vol_actual = objetivo;
        self.pista = Pista::Casino;
    }

    /// Corte seco: la musica del casino se va de golpe, sin fade, y entra el
    /// drone. El corte es a proposito, es el momento en que se acaba la fiesta.
    pub fn laberinto(&mut self) {
        if self.pista == Pista::Drone {
            return;
        }
        if let Some(m) = &self.musica {
            m.stop_stream();
        }
        if let Some(d) = &self.drone {
            d.set_volume(VOL_DRONE_BASE);
            d.play_stream();
        }
        self.vol_actual = VOL_DRONE_BASE;
        self.vol_objetivo = VOL_DRONE_BASE;
        self.pista = Pista::Drone;
        self.t_latido = 0.0;
        self.t_pasos = 0.0;
    }

    /// victoria y derrota: se para todo
    pub fn silencio(&mut self) {
        if self.pista == Pista::Nada {
            return;
        }
        if let Some(m) = &self.musica {
            m.stop_stream();
        }
        if let Some(d) = &self.drone {
            d.stop_stream();
        }
        self.pista = Pista::Nada;
    }

    // ---------------------------------------------- por frame

    /// VA CADA FRAME EN EL BUCLE PRINCIPAL. Si el stream no se actualiza, la
    /// musica se corta a los pocos segundos: raylib rellena el buffer aca.
    /// update_stream y set_volume toman &self, asi que el &mut de esta firma es
    /// solo por el fade y los relojes, no por raylib.
    pub fn actualizar(&mut self, dt: f32) {
        // el volumen se acerca al objetivo a velocidad constante
        let paso = VEL_VOLUMEN * dt;
        if self.vol_actual < self.vol_objetivo {
            self.vol_actual = (self.vol_actual + paso).min(self.vol_objetivo);
        } else if self.vol_actual > self.vol_objetivo {
            self.vol_actual = (self.vol_actual - paso).max(self.vol_objetivo);
        }

        match self.pista {
            Pista::Casino => {
                if let Some(m) = &self.musica {
                    m.set_volume(self.vol_actual);
                    m.update_stream();
                }
            }
            Pista::Drone => {
                if let Some(d) = &self.drone {
                    d.set_volume(self.vol_actual);
                    d.update_stream();
                }
            }
            Pista::Nada => {}
        }
    }

    /// Intensidad del laberinto segun que tan encima esta la sombra. `cerca` va
    /// 0 lejos y 1 encima, y sale de main con est.dist_enemigo(): el audio no
    /// entra en Estado.
    ///
    /// El latido y los pasos van por acumulador, no por frame: a 60 fps
    /// dispararlos cada frame serian 60 golpes por segundo.
    pub fn tension(&mut self, dt: f32, persiguiendo: bool, cerca: f32) {
        if !persiguiendo {
            // la sombra no anda suelta: drone al minimo y nada de latido
            self.vol_objetivo = VOL_DRONE_BASE;
            self.t_latido = 0.0;
            self.t_pasos = 0.0;
            return;
        }

        let cerca = cerca.clamp(0.0, 1.0);
        self.vol_objetivo = VOL_DRONE_BASE + VOL_DRONE_CERCA * cerca;

        self.t_latido += dt;
        let intervalo = LATIDO_LEJOS + (LATIDO_CERCA - LATIDO_LEJOS) * cerca;
        if self.t_latido >= intervalo {
            self.t_latido = 0.0;
            // el golpe tambien pega mas fuerte de cerca
            tocar(&self.latido, VOL_LATIDO * (0.4 + 0.6 * cerca));
        }

        self.t_pasos += dt;
        let intervalo = PASOS_LEJOS + (PASOS_CERCA - PASOS_LEJOS) * cerca;
        if self.t_pasos >= intervalo {
            self.t_pasos = 0.0;
            tocar(&self.pasos, VOL_PASOS * cerca);
        }
    }

    // ---------------------------------------------- efectos de la maquina

    /// El clunk de la palanca: suena al apretar la tecla, cuando la palanca
    /// empieza a bajar. Los 0.10s que tarda en tocar fondo son justo el desfase
    /// que hace sentir que la palanca causo el giro.
    pub fn palanca(&self) {
        tocar(&self.palanca, 1.0);
    }

    /// Suena una sola vez, en el frame en que arranca el giro. El que llama
    /// es el mismo punto que dispara AnimRodillos::iniciar(), no la fase: la
    /// fase se reasigna cada frame y esto sonaria 60 veces por segundo.
    pub fn slot(&self) {
        tocar(&self.slot, 1.0);
    }

    pub fn rodillo_para(&self) {
        tocar(&self.rodillo, 1.0);
    }

    pub fn pago(&self) {
        tocar(&self.pago, 1.0);
    }

    pub fn fallo(&self) {
        tocar(&self.fallo, 1.0);
    }
}
