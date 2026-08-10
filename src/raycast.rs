// raycast.rs — lanzamiento de rayos e impacto

use crate::mapa::{char_en, es_pared};

pub const PASO_RAYO: f32 = 0.02;
pub const MAX_DIST: f32 = 40.0;

pub struct Impacto {
    pub d: f32,
    pub ch: char,
    pub tx: f32,
}

/// Marcher de paso fijo: avanza de a PASO_RAYO y consulta la celda en cada
/// muestra. La orientacion de la cara golpeada se deduce de que indice de celda
/// cambio en ese paso.
///
/// LIMITACION CONOCIDA: con paso fijo, un solo paso puede cruzar una linea de
/// grilla vertical y una horizontal a la vez. Cuando pasa, `vertical` gana por
/// orden de evaluacion y la cara se clasifica como normal +-x aunque en
/// realidad sea horizontal, asi que esa columna toma la coordenada de textura
/// del eje equivocado.
///
/// El sintoma son columnas de textura sueltas, mal mapeadas, justo en las
/// esquinas de pared. Es de una estaca de ancho y solo aparece en la esquina,
/// por eso pasa desapercibido casi siempre.
///
/// No se arregla achicando PASO_RAYO: eso lo hace mas raro, no lo elimina, y
/// cuesta mas muestras por rayo. La solucion de verdad es pasar a DDA, que
/// avanza de linea de grilla en linea de grilla y por construccion sabe cual de
/// las dos cruzo, sin ambiguedad ni muestreo.
pub fn lanzar(grid: &[Vec<char>], x: f32, y: f32, ang: f32) -> Impacto {
    let (dx, dy) = (ang.cos(), ang.sin());
    let mut t = 0.0f32;
    // solo se sigue la celda en x: alcanza para saber la orientacion de la cara
    let mut cx_ant = x.floor();

    while t < MAX_DIST {
        t += PASO_RAYO;
        let hx = x + dx * t;
        let hy = y + dy * t;
        let ch = char_en(grid, hx, hy);

        if es_pared(ch) {
            // cambio el indice de celda en x = el rayo cruzo una linea de
            // grilla vertical = la cara golpeada tiene normal +-x
            let cx = hx.floor();
            let vertical = cx != cx_ant;

            // La cara define sobre que eje corre la textura: la de normal +-x
            // se extiende a lo largo de y, asi que le toca hy; la horizontal al
            // reves.
            //
            // El else cubre dos casos con la misma formula a proposito, no le
            // falta una rama: o la cara es horizontal y le toca hx, o el rayo
            // no cruzo ninguna linea de grilla, que solo pasa si arranca
            // adentro de una pared (imposible mientras libre() siga cuidando el
            // movimiento) y ahi cualquier valor sirve.
            let mut tx = if vertical {
                hy - hy.floor()
            } else {
                hx - hx.floor()
            };

            if (vertical && dx > 0.0) || (!vertical && dy < 0.0) {
                tx = 1.0 - tx;
            }
            return Impacto { d: t, ch, tx };
        }

        cx_ant = hx.floor();
    }
    Impacto {
        d: MAX_DIST,
        ch: '+',
        tx: 0.0,
    }
}
