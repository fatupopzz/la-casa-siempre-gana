// gen.rs — generacion de laberintos
//
// Backtracker con pila: perfecto (un solo camino entre dos celdas cualquiera,
// sin ciclos), que es lo que hace que perderse cueste y que el BFS del
// perseguidor tenga siempre una ruta unica que seguir.
//
// La salida es el MISMO formato de texto que mapas/*.txt: '#' pared, ' ' paso,
// 'A' entrada, 'B' salida y una 'M' de maquina. Se comparte entre el juego (el
// modo infinito genera cada piso en runtime) y el binario `gen`, que lo escribe
// a un archivo.

use std::time::{SystemTime, UNIX_EPOCH};

const ENTRADA: char = 'A';
const SALIDA: char = 'B';
const MAQUINA: char = 'M';
const PARED: char = '#';
const PASO: char = ' ';

/// xorshift64 sembrado con el reloj. No hace falta nada mejor: lo unico que se
/// le pide es que dos partidas seguidas no salgan iguales, y no se guarda
/// semilla porque los pisos del modo infinito no se rejuegan.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64;
        // el estado no puede ser cero: xorshift se queda clavado ahi para siempre
        Rng(n | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

struct Maze {
    w: usize,
    h: usize,
    right: Vec<bool>,
    down: Vec<bool>,
}

impl Maze {
    fn idx(&self, r: usize, c: usize) -> usize {
        r * self.w + c
    }

    fn generate(w: usize, h: usize, rng: &mut Rng) -> Self {
        let mut m = Maze {
            w,
            h,
            right: vec![true; w * h],
            down: vec![true; w * h],
        };
        let mut visited = vec![false; w * h];
        let mut stack = vec![(0usize, 0usize)];
        visited[0] = true;

        while let Some(&(r, c)) = stack.last() {
            let mut op: Vec<usize> = Vec::with_capacity(4);
            if r > 0 && !visited[m.idx(r - 1, c)] {
                op.push(0);
            }
            if c + 1 < w && !visited[m.idx(r, c + 1)] {
                op.push(1);
            }
            if r + 1 < h && !visited[m.idx(r + 1, c)] {
                op.push(2);
            }
            if c > 0 && !visited[m.idx(r, c - 1)] {
                op.push(3);
            }

            if op.is_empty() {
                stack.pop();
                continue;
            }

            let dir = op[rng.below(op.len())];
            let (nr, nc) = match dir {
                0 => (r - 1, c),
                1 => (r, c + 1),
                2 => (r + 1, c),
                _ => (r, c - 1),
            };

            match dir {
                0 => {
                    let i = m.idx(nr, nc);
                    m.down[i] = false;
                }
                1 => {
                    let i = m.idx(r, c);
                    m.right[i] = false;
                }
                2 => {
                    let i = m.idx(r, c);
                    m.down[i] = false;
                }
                _ => {
                    let i = m.idx(nr, nc);
                    m.right[i] = false;
                }
            }

            let ni = m.idx(nr, nc);
            visited[ni] = true;
            stack.push((nr, nc));
        }
        m
    }

    fn to_grid(&self, cell_w: usize) -> Vec<Vec<char>> {
        let rows = 2 * self.h + 1;
        let cols = cell_w * self.w + 1;
        let mut g = vec![vec![PARED; cols]; rows];

        // las filas impares son las celdas: se abren enteras salvo los postes
        for fila in g.iter_mut().skip(1).step_by(2) {
            for (c, celda) in fila.iter_mut().enumerate() {
                if c % cell_w != 0 {
                    *celda = PASO;
                }
            }
        }

        for r in 0..self.h {
            for c in 0..self.w {
                let i = self.idx(r, c);
                if !self.right[i] && c + 1 < self.w {
                    g[2 * r + 1][cell_w * (c + 1)] = PASO;
                }
                if !self.down[i] && r + 1 < self.h {
                    for k in 1..cell_w {
                        g[2 * r + 2][cell_w * c + k] = PASO;
                    }
                }
            }
        }

        g[1][cell_w / 2] = ENTRADA;
        g[2 * self.h - 1][cell_w * (self.w - 1) + cell_w / 2] = SALIDA;
        g
    }
}

/// Mete la maquina en un pedazo de pared que de a un pasillo. Va sobre pared y
/// no sobre paso a proposito: 'M' es solida (es_pared la cuenta), asi que
/// pisarla no se puede y ponerla en un pasillo taparia el laberinto. Como no
/// abre ni cierra caminos, el laberinto sigue siendo el mismo.
///
/// Se descartan las paredes pegadas a la entrada y a la salida: la maquina ahi
/// se jala sin caminar nada, y la idea es que haya que ir a buscarla.
fn poner_maquina(g: &mut [Vec<char>], rng: &mut Rng) {
    let filas = g.len();
    let cols = g[0].len();
    let mut candidatas = Vec::new();

    for r in 1..filas - 1 {
        for c in 1..cols - 1 {
            if g[r][c] != PARED {
                continue;
            }
            let vecinas = [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)];
            let da_a_pasillo = vecinas.iter().any(|&(vr, vc)| g[vr][vc] == PASO);
            let junto_a_punta = vecinas
                .iter()
                .any(|&(vr, vc)| g[vr][vc] == ENTRADA || g[vr][vc] == SALIDA);
            if da_a_pasillo && !junto_a_punta {
                candidatas.push((r, c));
            }
        }
    }

    // un laberinto de 2x2 celdas puede no dejar ninguna candidata; ahi se queda
    // sin maquina en vez de forzarla en un lugar que rompa el mapa
    if let Some(&(r, c)) = candidatas.get(rng.below(candidatas.len().max(1))) {
        g[r][c] = MAQUINA;
    }
}

/// Genera un laberinto y lo devuelve como texto, listo para Estado o para
/// escribir a un .txt. `celdas_w` x `celdas_h` son celdas, no caracteres: el
/// grid que sale mide `cell_w * celdas_w + 1` por `2 * celdas_h + 1`.
///
/// `cell_w` = 2 da bloques cuadrados, que es lo que quiere el raycaster; 4 se
/// ve mejor como ASCII en la terminal y solo lo usa el binario.
pub fn generar(celdas_w: usize, celdas_h: usize, cell_w: usize) -> String {
    let mut rng = Rng::new();
    let maze = Maze::generate(celdas_w.max(2), celdas_h.max(2), &mut rng);
    let mut grid = maze.to_grid(cell_w.max(2));
    poner_maquina(&mut grid, &mut rng);

    let mut out = String::new();
    for fila in &grid {
        out.extend(fila.iter());
        out.push('\n');
    }
    out
}
