# La Casa Siempre Gana

Ray caster en primera persona con temática de horror y casino, desarrollado en Rust con raylib-rs para el curso CC2018 Gráficas por Computadora (UVG).

Link al video de demostracion: [Ver demo en YouTube](https://youtu.be/3fGIWCZBxe0)
Link al video de demostracion CON SONIDO : [Ver demo en YouTube](https://youtu.be/d6gD7s6qLCg)

Una máquina tragamonedas en un cuarto opresivo. Tenés una cuota que cumplir y un número limitado de giros para lograrlo. Si la cumplís, salís. Si no, las paredes se abren en un laberinto y algo empieza a seguirte.

<img width="975" height="624" alt="image" src="https://github.com/user-attachments/assets/6347f906-8f30-4e78-8404-9d05f0a49108" />

<img width="975" height="624" alt="image" src="https://github.com/user-attachments/assets/37144cbc-83ad-4798-9558-e8f62f99df24" />

<img width="975" height="624" alt="image" src="https://github.com/user-attachments/assets/cce12943-f30e-4df7-a4ff-f10e8aab7492" />




## Controles

### Teclado

| Tecla | Acción |
|---|---|
| W / S | Caminar adelante / atrás |
| A / D | Strafe izquierda / derecha |
| Q / E | Strafe (alternativo) |
| Mouse | Rotar la cámara |
| F | Interactuar (jalar palanca / entrar a máquina en el laberinto) |
| Enter | Confirmar selección |
| T | Alternar texturas |
| M | Alternar vista 2D / 3D |
| 1 / 2 / 3 | Seleccionar piso (en la bienvenida) |
| R | Reiniciar (en pantallas finales) |
| Esc | Salir |

### Gamepad

| Input | Acción |
|---|---|
| Stick izquierdo | Caminar y strafe |
| Stick derecho (horizontal) | Rotar |
| Botón sur (A / ×) | Interactuar |
| Botón este (B / ○) | Confirmar |
| D-pad | Seleccionar piso |
| Cualquier botón | Reiniciar (en pantallas finales) |

## Cómo compilar

### Requisitos

- Rust (edición 2021 o posterior)
- Dependencias de sistema de raylib (se compilan automáticamente desde el crate)

En macOS puede necesitar Xcode Command Line Tools. En Linux, instalar los paquetes de desarrollo de X11/Wayland y OpenGL que pida el compilador.

### Compilar y correr

```bash
git clone https://github.com/USUARIO/maze-runner.git
cd maze-runner
cargo run --release
```

Siempre usar `--release`. En modo debug el raycasting baja a fps inaceptables.

## Estructura del proyecto

```
src/
├── main.rs       — bucle principal, audio, máquina de escenas
├── juego.rs      — transiciones entre escenas
├── mapa.rs       — carga de mapas, colisión, celdas
├── raycast.rs    — DDA raycaster (distancia radial)
├── estado.rs     — estado del juego, BFS del perseguidor, fog of war
└── render.rs     — render 3D, sombra procedural, minimapa, post-proceso
```

## Pisos

| Piso | Cuota | Giros | Probabilidad de éxito |
|---|---|---|---|
| 1 | 65 | 20 | ~44% |
| 2 | 70 | 16 | ~21% |
| 3 | 55 | 12 | ~19% |

Hay un modo infinito que escala la dificultad indefinidamente.

## Créditos

Proyecto individual para CC2018 Gráficas por Computadora, Universidad del Valle de Guatemala.

Todos los assets gráficos (texturas, sprites, placas, gabinete, símbolos) fueron generados con scripts de Python/Pillow escritos para el proyecto. La fuente bitmap es de 5×7 píxeles, diseñada a mano.

Audio: drone y latido generados para el proyecto. Efectos de la máquina recortados con ffmpeg.
