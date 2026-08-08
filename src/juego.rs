// juego.rs — escenas del juego

#[derive(Clone, Copy, PartialEq)]
pub enum Escena {
    Bienvenida,
    Maquina,
    Jugando,
    Victoria,
    Derrota,
}
