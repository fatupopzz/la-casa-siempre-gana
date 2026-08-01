// mapa.rs — carga del grid, consulta de celdas, colision, BFS

use std::collections::VecDeque;
use std::fs;

pub const RADIO: f32 = 0.24;

pub fn cargar(path: &str) -> Vec<Vec<char>> {
    let txt = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("no encontre '{}'. Corre: cargo run --bin gen", path));

    let mut grid: Vec<Vec<char>> = txt
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().collect())
        .collect();

    let ancho = grid.iter().map(|f| f.len()).max().unwrap_or(0);
    for fila in grid.iter_mut() {
        fila.resize(ancho, ' ');
    }
    grid
}

pub fn buscar(grid: &[Vec<char>], objetivo: char) -> Option<(usize, usize)> {
    for (r, fila) in grid.iter().enumerate() {
        for (c, ch) in fila.iter().enumerate() {
            if *ch == objetivo {
                return Some((r, c));
            }
        }
    }
    None
}

pub fn es_pared(ch: char) -> bool {
    matches!(ch, '+' | '-' | '|')
}

pub fn char_en(grid: &[Vec<char>], x: f32, y: f32) -> char {
    if x < 0.0 || y < 0.0 {
        return '+';
    }
    let (c, r) = (x as usize, y as usize);
    if r >= grid.len() || c >= grid[0].len() {
        return '+';
    }
    grid[r][c]
}

pub fn libre(grid: &[Vec<char>], x: f32, y: f32) -> bool {
    !es_pared(char_en(grid, x - RADIO, y - RADIO))
        && !es_pared(char_en(grid, x + RADIO, y - RADIO))
        && !es_pared(char_en(grid, x - RADIO, y + RADIO))
        && !es_pared(char_en(grid, x + RADIO, y + RADIO))
}

/// BFS: distancia de cada celda hasta (r0,c0). -1 = pared o inalcanzable.
pub fn campo_desde(grid: &[Vec<char>], r0: usize, c0: usize) -> Vec<i32> {
    let filas = grid.len();
    let cols = grid[0].len();
    let mut d = vec![-1i32; filas * cols];

    if es_pared(grid[r0][c0]) {
        return d;
    }

    let mut q = VecDeque::new();
    d[r0 * cols + c0] = 0;
    q.push_back((r0, c0));

    while let Some((r, c)) = q.pop_front() {
        let base = d[r * cols + c];
        for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr < 0 || nc < 0 || nr >= filas as i32 || nc >= cols as i32 {
                continue;
            }
            let (nr, nc) = (nr as usize, nc as usize);
            if es_pared(grid[nr][nc]) || d[nr * cols + nc] != -1 {
                continue;
            }
            d[nr * cols + nc] = base + 1;
            q.push_back((nr, nc));
        }
    }
    d
}
