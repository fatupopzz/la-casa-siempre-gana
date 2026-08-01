// Generador de laberinto -> maze.txt
// uso: cargo run --bin gen -- [ancho] [alto] [cell_w]
//   cell_w = 2  -> bloques cuadrados (recomendado para raylib)
//   cell_w = 4  -> se ve mas bonito como ASCII en la terminal

use std::env;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const START: char = 'A';
const GOAL: char = 'B';

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as u64;
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
        let mut g = vec![vec![' '; cols]; rows];

        for r in 0..rows {
            for c in 0..cols {
                g[r][c] = if r % 2 == 0 {
                    if c % cell_w == 0 {
                        '+'
                    } else {
                        '-'
                    }
                } else if c % cell_w == 0 {
                    '|'
                } else {
                    ' '
                };
            }
        }

        for r in 0..self.h {
            for c in 0..self.w {
                let i = self.idx(r, c);
                if !self.right[i] && c + 1 < self.w {
                    g[2 * r + 1][cell_w * (c + 1)] = ' ';
                }
                if !self.down[i] && r + 1 < self.h {
                    for k in 1..cell_w {
                        g[2 * r + 2][cell_w * c + k] = ' ';
                    }
                }
            }
        }

        g[1][cell_w / 2] = START;
        g[2 * self.h - 1][cell_w * (self.w - 1) + cell_w / 2] = GOAL;
        g
    }
}

fn main() {
    let a: Vec<String> = env::args().collect();
    let w: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(15);
    let h: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(11);
    let cell_w: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(2);

    let mut rng = Rng::new();
    let maze = Maze::generate(w.max(2), h.max(2), &mut rng);
    let grid = maze.to_grid(cell_w.max(2));

    let mut out = String::new();
    for fila in &grid {
        out.extend(fila.iter());
        out.push('\n');
    }

    fs::write("maze.txt", &out).expect("no se pudo escribir maze.txt");
    print!("{}", out);
    println!("-> maze.txt ({}x{} celdas, cell_w={})", w, h, cell_w);
}
