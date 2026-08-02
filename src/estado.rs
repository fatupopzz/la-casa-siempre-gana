// estado.rs — estado del juego, movimiento del jugador, perseguidor

use crate::mapa::{cargar, buscar, es_pared, char_en, libre, campo_desde};

const VEL_ENEMIGO: f32 = 3.8;
const PASOS_SPAWN: i32 = 30;
const RECALC: f32 = 0.25;
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
}

impl Estado {
    pub fn nuevo(path: &str) -> Self {
        let grid = cargar(path);
        let (pr, pc) = buscar(&grid, 'A').expect("el maze.txt no tiene 'A'");
        let cols = grid[0].len();
       

        let campo = campo_desde(&grid, pr, pc);
        let mut spawn = None;
        let mut mejor_dif = i32::MAX;
        for (i, v) in campo.iter().enumerate() {
            if *v < 0 {
                continue;
            }
            let dif = (*v - PASOS_SPAWN).abs();
            if dif < mejor_dif {
                mejor_dif = dif;
                spawn = Some(i);
            }
        }
        let idx = spawn.unwrap_or(pr * cols + pc);
        let (er, ec) = (idx / cols, idx % cols);

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

    pub fn perseguir(&mut self, dt: f32) {
        self.anim_t += dt;
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
            let paso = VEL_ENEMIGO * dt;
            self.ex += dx / len * paso;
            self.ey += dy / len * paso;
        }

        if self.dist_enemigo() < DIST_ATRAPA {
            self.atrapado = true;
        }
    }
}
