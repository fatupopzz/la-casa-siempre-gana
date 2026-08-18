// Generador de laberinto -> maze.txt
// uso: cargo run --bin gen -- [ancho] [alto] [cell_w]
//   cell_w = 2  -> bloques cuadrados (recomendado para raylib)
//   cell_w = 4  -> se ve mas bonito como ASCII en la terminal
//
// La generacion no vive aca sino en src/gen.rs, que es el mismo modulo que usa
// el modo infinito del juego. Aca solo se parsean los argumentos y se escribe
// el archivo: asi el laberinto que sale por linea de comandos es exactamente el
// que se juega, y no dos generadores que se van separando con el tiempo.
//
// El #[path] es para compartir el archivo sin tener que convertir el proyecto
// en biblioteca: src/bin/*.rs es su propio crate y no ve los modulos de
// main.rs, asi que se compila una segunda copia del mismo fuente.
#[path = "../gen.rs"]
mod gen;

use std::env;
use std::fs;

fn main() {
    let a: Vec<String> = env::args().collect();
    let w: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(15);
    let h: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(11);
    let cell_w: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(2);

    let out = gen::generar(w, h, cell_w);

    fs::write("maze.txt", &out).expect("no se pudo escribir maze.txt");
    print!("{}", out);
    println!("-> maze.txt ({}x{} celdas, cell_w={})", w, h, cell_w);
}
