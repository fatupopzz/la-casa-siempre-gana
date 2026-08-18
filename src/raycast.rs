// raycast.rs — lanzamiento de rayos e impacto

use crate::mapa::{char_en, es_pared};

pub const MAX_DIST: f32 = 40.0;

pub struct Impacto {
    pub d: f32,
    pub ch: char,
    pub tx: f32,
}

/// Lanza un rayo por DDA: en vez de muestrear cada tanto, salta de linea de
/// grilla en linea de grilla, avanzando en cada vuelta a la proxima
/// interseccion, la vertical o la horizontal, la que caiga mas cerca. El mismo
/// `if` que elige por donde avanzar es el que dice que orientacion tiene la
/// cara golpeada, asi que no hay ambiguedad al clasificarla. Da la distancia
/// exacta al impacto y visita solo las celdas que el rayo realmente atraviesa.
///
/// El caso delicado es el empate exacto dist_x == dist_y, que fue el bug real
/// de esta implementacion: se resuelve cruzando el vertice en diagonal, ver el
/// comentario en la rama.
///
/// `d` sale RADIAL: la correccion de fisheye la hace render_3d multiplicando
/// por cos(ang - est.a), y render_2d necesita la radial para plotear. Devolver
/// la perpendicular desde aca la corregiria dos veces.
pub fn lanzar_dda(grid: &[Vec<char>], x: f32, y: f32, ang: f32) -> Impacto {
    lanzar_dda_visitando(grid, x, y, ang, |_, _| {})
}

/// Igual que lanzar_dda(), pero avisa por `visitar` cada celda que el rayo
/// pisa, en orden y arrancando por la del origen. La usa el fog of war del
/// minimapa: el DDA ya sabe exactamente que celdas atraviesa, asi que revelar
/// lo que se ve no cuesta un segundo recorrido.
///
/// La celda del impacto tambien se visita: la pared que corta la vista se ve, y
/// si no se revelara el minimapa quedaria con agujeros justo en los bordes.
///
/// El visitante es generico y no un `&mut dyn`: con el cierre vacio de
/// lanzar_dda() el monomorfizado lo borra entero, asi que la version que no
/// revela nada no paga nada.
pub fn lanzar_dda_visitando(
    grid: &[Vec<char>],
    x: f32,
    y: f32,
    ang: f32,
    mut visitar: impl FnMut(i32, i32),
) -> Impacto {
    let (dx, dy) = (ang.cos(), ang.sin());
    let mut celda_x = x.floor() as i32;
    let mut celda_y = y.floor() as i32;

    // la celda del origen cuenta como visitada, y se avisa antes del caso
    // degenerado para que salga por los dos caminos y no solo por el normal
    visitar(celda_x, celda_y);

    // Caso degenerado: el rayo arranca adentro de una pared. No pasa mientras
    // libre() cuide el movimiento, pero si pasara el DDA saldria a buscar la
    // pared SIGUIENTE y dibujaria una lejana con el jugador incrustado en otra.
    // Se resuelve devolviendo la pared en la cara: distancia cero, cara
    // horizontal, y la misma regla de espejeo que el resto.
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
            // elegir una, y se elige la vertical. Cual de las dos sea importa
            // menos que el hecho de que sea siempre la misma: si alternara,
            // dos rayos vecinos que caen los dos en el vertice sacarian la
            // coordenada de textura de ejes distintos.
            vertical = true;
        }

        // el rayo se agota: mismo Impacto de fallo que si no hubiera pared
        if d >= MAX_DIST {
            return Impacto { d: MAX_DIST, ch: '+', tx: 0.0 };
        }

        // recien aca, y no apenas se avanzan los indices: la celda que cae mas
        // alla de MAX_DIST no se llega a mirar, asi que tampoco se visita. Se
        // visita exactamente lo mismo que se consulta.
        visitar(celda_x, celda_y);

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

            // espejeo segun de que lado se mira la pared, para que las caras
            // opuestas de una misma orientacion no salgan una espejada de la otra
            if (vertical && dx > 0.0) || (!vertical && dy < 0.0) {
                tx = 1.0 - tx;
            }
            return Impacto { d, ch, tx };
        }
    }
}
