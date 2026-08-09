// juego.rs — escenas del juego

#[derive(Clone, Copy, PartialEq)]
pub enum Escena {
    Bienvenida,
    Maquina,
    Jugando,
    /// se pego la cuota: la partida se cierra en verde
    Exito,
    Victoria,
    Derrota,
}
