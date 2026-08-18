// estado.rs — estado del juego, movimiento del jugador, perseguidor

use crate::mapa::{cargar, buscar, char_en, libre, campo_desde, desde_texto};
use crate::raycast::{lanzar_dda_visitando, Impacto};

// La sombra sale EN LA SALIDA y arranca a venir hacia vos de una. Suena raro
// que nazca en la meta, pero es al reves de lo que parece: como te persigue,
// abandona la salida en el primer segundo y te la deja libre. Lo que no se
// puede es que se quede parada ahi esperandote, y por eso no lleva espera en la
// entrada forzada.
const RECALC: f32 = 0.7;
const DIST_ATRAPA: f32 = 0.5;

pub struct Estado {
    pub grid: Vec<Vec<char>>,
    pub x: f32,
    pub y: f32,
    pub a: f32,
    pub modo3d: bool,
    pub gano: bool,
    pub ex: f32,
    pub ey: f32,
    pub campo: Vec<i32>,
    pub t_recalc: f32,
    pub atrapado: bool,
    pub anim_t: f32,
    pub persiguiendo: bool,
    pub vel_enemigo: f32,
    /// segundos que faltan para que la sombra salga. Solo lo usa la entrada
    /// voluntaria al laberinto; en la forzada arranca en 0 y no hace nada.
    pub t_espera: f32,
    /// Fog of war del minimapa: que celdas ya vio el jugador. Plano y
    /// row-major, `fila * cols + col`, igual que `campo`.
    ///
    /// No se resetea a mano en ningun lado y no hace falta: cada piso y cada
    /// reinicio construyen un Estado nuevo por nuevo(), asi que el revelado
    /// arranca en false junto con el resto. Si alguna vez se recicla el Estado
    /// en vez de reconstruirlo, esto hay que limpiarlo ahi.
    pub revelado: Vec<bool>,
}

impl Estado {
    /// la velocidad de la sombra la manda el ConfigPiso del piso que se entra:
    /// va como parametro para que no exista un default global que se pueda
    /// quedar viejo. Hoy es MENOR que la del jugador, a proposito.
    pub fn nuevo(path: &str, vel_enemigo: f32) -> Self {
        Self::desde_grid(cargar(path), vel_enemigo)
    }

    /// Igual que nuevo() pero desde el texto del mapa en vez de un archivo: lo
    /// usa el modo infinito, que genera cada piso en runtime.
    pub fn de_texto(txt: &str, vel_enemigo: f32) -> Self {
        Self::desde_grid(desde_texto(txt), vel_enemigo)
    }

    fn desde_grid(grid: Vec<Vec<char>>, vel_enemigo: f32) -> Self {
        let (pr, pc) = buscar(&grid, 'A').expect("el maze.txt no tiene 'A'");
        let campo = campo_desde(&grid, pr, pc);
        // arranca en la salida; si el mapa no tiene 'B' cae en la entrada
        let (er, ec) = buscar(&grid, 'B').unwrap_or((pr, pc));
        let celdas = grid.len() * grid[0].len();

        Estado {
            grid,
            x: pc as f32 + 0.5,
            y: pr as f32 + 0.5,
            a: 0.0,
            modo3d: false,
            gano: false,
            ex: ec as f32 + 0.5,
            ey: er as f32 + 0.5,
            campo,
            t_recalc: 0.0,
            atrapado: false,
            anim_t: 0.0,
            persiguiendo: false,
            vel_enemigo,
            t_espera: 0.0,
            revelado: vec![false; celdas],
        }
    }

    pub fn cols(&self) -> i32 {
        self.grid[0].len() as i32
    }

    pub fn filas(&self) -> i32 {
        self.grid.len() as i32
    }

    pub fn dist_enemigo(&self) -> f32 {
        ((self.ex - self.x).powi(2) + (self.ey - self.y).powi(2)).sqrt()
    }

    /// Revela la celda que el jugador esta pisando. Se llama por frame y no
    /// desde avanzar(): avanzar() solo corre si hay tecla apretada, asi que la
    /// celda de arranque no se revelaria hasta dar el primer paso.
    pub fn revelar_jugador(&mut self) {
        self.revelar_celda(self.x as i32, self.y as i32);
    }

    /// Tira un rayo y revela TODAS las celdas que atraviesa, no solo la pared
    /// donde termina: asi el minimapa muestra lo que el jugador ve y no nada
    /// mas lo que piso. Devuelve el mismo Impacto que lanzar_dda() para que
    /// render_3d dibuje la estaca con esta misma pasada, sin tirar el rayo dos
    /// veces.
    ///
    /// Vive aca y no en render.rs por el prestamo: el cierre necesita
    /// `&mut self.revelado` mientras el DDA lee `&self.grid`. Desde adentro se
    /// desestructura y los dos campos salen por separado; desde afuera, un
    /// est.revelar_celda() dentro del cierre tomaria el struct entero y
    /// chocaria con el &est.grid del mismo llamado.
    pub fn revelar_rayo(&mut self, ang: f32) -> Impacto {
        let (cols, filas) = (self.cols(), self.filas());
        let (x, y) = (self.x, self.y);
        let Estado { grid, revelado, .. } = self;

        lanzar_dda_visitando(grid, x, y, ang, |cx, cy| {
            if cx >= 0 && cy >= 0 && cx < cols && cy < filas {
                revelado[(cy * cols + cx) as usize] = true;
            }
        })
    }

    /// El rayo puede salirse del mapa (char_en() devuelve '+' afuera y ahi
    /// corta), asi que la celda llega sin garantia de estar adentro y el
    /// chequeo de rango va aca, en un solo lugar.
    fn revelar_celda(&mut self, cx: i32, cy: i32) {
        let (cols, filas) = (self.cols(), self.filas());
        if cx >= 0 && cy >= 0 && cx < cols && cy < filas {
            self.revelado[(cy * cols + cx) as usize] = true;
        }
    }

    /// Si la celda (col, fila) ya se vio. Fuera del mapa devuelve false, que es
    /// lo que espera el minimapa: lo que no existe tampoco se dibuja.
    pub fn visto(&self, cx: i32, cy: i32) -> bool {
        let (cols, filas) = (self.cols(), self.filas());
        cx >= 0
            && cy >= 0
            && cx < cols
            && cy < filas
            && self.revelado[(cy * cols + cx) as usize]
    }

    pub fn avanzar(&mut self, dx: f32, dy: f32) {
        if libre(&self.grid, self.x + dx, self.y) {
            self.x += dx;
        }
        if libre(&self.grid, self.x, self.y + dy) {
            self.y += dy;
        }
        if char_en(&self.grid, self.x, self.y) == 'B' {
            self.gano = true;
        }
    }

    /// Adelanta los relojes del enemigo sin moverlo: la animacion, el contador
    /// de recalculo del BFS y la espera de salida.
    ///
    /// Va SEPARADO del movimiento porque hay una escena, la maquina, donde el
    /// tiempo tiene que seguir corriendo pero la sombra no puede avanzar. Si en
    /// vez de esto se saltara perseguir() entera, se congelarian tambien
    /// t_espera y anim_t: la sombra nunca terminaria de despertarse mientras
    /// jugas al tragamonedas, y al salir arrancaria el contador de cero.
    pub fn correr_relojes(&mut self, dt: f32) {
        self.anim_t += dt;
        self.t_recalc -= dt;

        // la ventaja de haber salido por cuenta propia: la sombra todavia no
        // aparecio. Mientras corre esto no hay persecucion ni apagon.
        if self.t_espera > 0.0 {
            self.t_espera -= dt;
            if self.t_espera <= 0.0 {
                self.persiguiendo = true;
            }
        }
    }

    /// Mueve a la sombra un paso hacia el jugador y cobra si lo alcanzo.
    ///
    /// Ojo: DIST_ATRAPA se chequea aca adentro, o sea que donde no se llama a
    /// perseguir() tampoco se puede morir. Es justo lo que se busca en la
    /// maquina, donde el jugador no se puede mover para esquivar.
    pub fn perseguir(&mut self, dt: f32) {
        self.correr_relojes(dt);

        if !self.persiguiendo { return; }
        if self.t_recalc <= 0.0 {
            let (pr, pc) = (self.y as usize, self.x as usize);
            self.campo = campo_desde(&self.grid, pr, pc);
            self.t_recalc = RECALC;
        }

        let cols = self.grid[0].len();
        let filas = self.grid.len();
        let (er, ec) = (
            (self.ey as usize).min(filas - 1),
            (self.ex as usize).min(cols - 1),
        );
        let actual = self.campo[er * cols + ec];

        let mut mejor: Option<(usize, usize, i32)> = None;
        for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nr = er as i32 + dr;
            let nc = ec as i32 + dc;
            if nr < 0 || nc < 0 || nr >= filas as i32 || nc >= cols as i32 {
                continue;
            }
            let (nr, nc) = (nr as usize, nc as usize);
            let v = self.campo[nr * cols + nc];
            if v < 0 {
                continue;
            }
            if mejor.is_none() || v < mejor.unwrap().2 {
                mejor = Some((nr, nc, v));
            }
        }

        let objetivo = match mejor {
            Some((nr, nc, v)) if actual < 0 || v < actual => (nc as f32 + 0.5, nr as f32 + 0.5),
            _ => (self.x, self.y),
        };

        let (dx, dy) = (objetivo.0 - self.ex, objetivo.1 - self.ey);
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.001 {
            let paso = self.vel_enemigo * dt;
            self.ex += dx / len * paso;
            self.ey += dy / len * paso;
        }

        if self.dist_enemigo() < DIST_ATRAPA {
            self.atrapado = true;
        }
    }
}
