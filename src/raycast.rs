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
///
/// REEMPLAZADA: los dos call sites de render.rs usan lanzar_dda(). Queda a mano
/// para poder comparar contra el render viejo mientras se prueba el juego. Se
/// borra despues de esa prueba.
///
/// Motivo del reemplazo, medido sobre 522.720 rayos en los tres mapas: el 99,8%
/// da lo mismo dentro del error del paso, pero en el 0,2% restante este marcher
/// saltea celdas que el rayo atraviesa por un tramo mas corto que PASO_RAYO
/// (esquinas cortadas al sesgo), y ademas clasifica mal la cara cuando un paso
/// cruza las dos lineas de grilla a la vez. El DDA no tiene ninguno de los dos
/// problemas porque calcula los cruces en forma cerrada en vez de muestrear.
#[allow(dead_code)]
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

/// Version DDA de lanzar(). Misma firma y mismo Impacto de salida, para poder
/// intercambiarlas sin tocar a los consumidores.
///
/// En vez de muestrear cada PASO_RAYO, salta de linea de grilla en linea de
/// grilla: en cada vuelta avanza a la proxima interseccion, la vertical o la
/// horizontal, la que caiga mas cerca. Eso elimina de raiz la ambiguedad de
/// esquina que documenta lanzar(), porque nunca cruza las dos a la vez: el
/// mismo `if` que elige por donde avanzar es el que dice que orientacion tiene
/// la cara. Ademas da la distancia exacta al impacto en vez de una redondeada
/// al paso, y visita solo las celdas que el rayo realmente atraviesa.
///
/// `d` sale RADIAL, igual que en lanzar(): la correccion de fisheye la hace
/// render_3d multiplicando por cos(ang - est.a), y render_2d necesita la radial
/// para plotear. Devolver la perpendicular desde aca la corregiria dos veces.
pub fn lanzar_dda(grid: &[Vec<char>], x: f32, y: f32, ang: f32) -> Impacto {
    let (dx, dy) = (ang.cos(), ang.sin());
    let mut celda_x = x.floor() as i32;
    let mut celda_y = y.floor() as i32;

    // Caso degenerado: el rayo arranca adentro de una pared. No pasa mientras
    // libre() cuide el movimiento, pero si pasara el DDA saldria a buscar la
    // pared SIGUIENTE y dibujaria una lejana con el jugador incrustado en otra.
    // Se resuelve como el marcher de paso fijo: pared en la cara, cara
    // horizontal, y la misma regla de espejeo.
    let ch_inicio = char_en(grid, x, y);
    if es_pared(ch_inicio) {
        let mut tx = x - x.floor();
        if dy < 0.0 {
            tx = 1.0 - tx;
        }
        return Impacto { d: 0.0, ch: ch_inicio, tx };
    }

    // Cuanto avanza el parametro t por cada linea de grilla completa de cada
    // eje. Como (dx, dy) es unitario, t es directamente distancia euclidea.
    //
    // Componente exactamente cero (rayo axial) da 1.0/0.0 = infinito, que en
    // f32 es un valor valido y no NaN: ese eje simplemente nunca gana la
    // comparacion de abajo. Lo que SI daria NaN es 0.0 * infinito, y por eso la
    // distancia inicial de ese eje se pone en infinito a mano en vez de
    // calcularla.
    let delta_x = (1.0 / dx).abs();
    let delta_y = (1.0 / dy).abs();

    let (paso_x, mut dist_x) = if dx == 0.0 {
        (0, f32::INFINITY)
    } else if dx < 0.0 {
        (-1, (x - celda_x as f32) * delta_x)
    } else {
        (1, (celda_x as f32 + 1.0 - x) * delta_x)
    };
    let (paso_y, mut dist_y) = if dy == 0.0 {
        (0, f32::INFINITY)
    } else if dy < 0.0 {
        (-1, (y - celda_y as f32) * delta_y)
    } else {
        (1, (celda_y as f32 + 1.0 - y) * delta_y)
    };

    loop {
        // Se avanza al cruce mas cercano. La distancia del cruce es la que
        // habia ANTES de sumarle el delta.
        //
        // El empate exacto necesita su propia rama y no es un detalle: pasa
        // cuando el rayo cruza justo por el vertice donde se juntan las dos
        // lineas de grilla (angulo diagonal exacto desde una posicion
        // alineada). Si ahi se avanzara un solo eje, se entraria a una celda
        // que el rayo apenas toca en un punto, de tramo cero, y si esa celda
        // es pared se reportaria un impacto contra algo que en realidad se roza
        // de costado: aparece una columna de pared mucho mas cerca de lo que
        // corresponde. Cruzando los dos ejes a la vez se pasa derecho a la
        // celda diagonal, que es a donde el rayo va de verdad.
        let (d, vertical);
        if dist_x < dist_y {
            d = dist_x;
            dist_x += delta_x;
            celda_x += paso_x;
            vertical = true;
        } else if dist_y < dist_x {
            d = dist_y;
            dist_y += delta_y;
            celda_y += paso_y;
            vertical = false;
        } else {
            d = dist_x;
            dist_x += delta_x;
            celda_x += paso_x;
            dist_y += delta_y;
            celda_y += paso_y;
            // en el vertice se tocan las dos caras a la vez, asi que hay que
            // elegir una: se elige la vertical, que es la que da lanzar() en
            // ese mismo caso (le cambian los dos indices y su `vertical` mira
            // el de x primero)
            vertical = true;
        }

        // corta igual que el marcher de paso fijo
        if d >= MAX_DIST {
            return Impacto { d: MAX_DIST, ch: '+', tx: 0.0 };
        }

        // se consulta por el centro de la celda y no por el punto de impacto:
        // ese punto cae justo sobre el borde y el redondeo lo puede tirar a la
        // celda de al lado. char_en() devuelve '+' fuera del mapa, asi que
        // salirse del grid corta el rayo igual que antes.
        let ch = char_en(grid, celda_x as f32 + 0.5, celda_y as f32 + 0.5);
        if es_pared(ch) {
            // La cara vertical (normal +-x) se extiende a lo largo de y, asi
            // que la textura corre sobre hy; la horizontal al reves. Del lado
            // por el que se cruzo, la coordenada es entera y no aporta.
            let mut tx = if vertical {
                let hy = y + dy * d;
                hy - hy.floor()
            } else {
                let hx = x + dx * d;
                hx - hx.floor()
            };

            // mismo espejeo que lanzar(), para que las caras opuestas de una
            // misma orientacion no salgan una espejada de la otra
            if (vertical && dx > 0.0) || (!vertical && dy < 0.0) {
                tx = 1.0 - tx;
            }
            return Impacto { d, ch, tx };
        }
    }
}
