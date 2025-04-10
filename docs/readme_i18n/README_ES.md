<h1 align="center">🍵 HTeaPot</h1>
<a href="https://www.flaticon.es/iconos-gratis/cafe" title="café iconos"></a>
<p align="center"><b>Una biblioteca de servidor HTTP ultrarrápida y minimalista creada con Rust</b></p>

<p align="center">
  <a href="https://crates.io/crates/hteapot"><img alt="Crates.io" src="https://img.shields.io/crates/v/hteapot.svg?style=flat-square"></a>
  <a href="https://docs.rs/hteapot"><img alt="Documentación" src="https://img.shields.io/docsrs/hteapot?style=flat-square"></a>
<!--   <a href="https://github.com/Az107/HTeaPot/actions"><img alt="Build Status" src="https://img.shields.io/github/actions/workflow/status/Az107/HTeaPot/rust.yml?branch=main&style=flat-square"></a> -->
  <a href="https://opensource.org/licenses/MIT"><img alt="Licencia: MIT" src="https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square"></a>
  <a href="https://github.com/Az107/HTeaPot" target="_blank"><img alt="Estrellas del repositorio de GitHub" src="https://img.shields.io/github/stars/Az107/HTeaPot"></a>
</p>

<p align="center">
  <a href="../../README.md">Inglés</a> |
  <a href="README_ES.md">Español</a>
</p>


HTeaPot es un servidor HTTP ligero y de alto rendimiento, con una biblioteca desarrollada en Rust. Está diseñado para ofrecer un rendimiento excepcional para aplicaciones web modernas, manteniendo una API sencilla e intuitiva.

## Características

### Rendimiento excepcional
- **Arquitectura de subprocesos**: Impulsada por un sistema de subprocesos diseñado a medida que gestiona **más de 70 000 solicitudes por segundo**
- **Consistencia bajo carga**: Mantiene un rendimiento estable incluso en escenarios de alta concurrencia
- **Resiliente**: Logra una **tasa de éxito casi perfecta del 100 %** con 200 respuestas correctas (_OK responses_) durante las pruebas de estrés

### Funcionalidad versátil
- **Servicio de archivos estáticos**: Sirve recursos estáticos de forma eficiente con una configuración mínima
- **Compatibilidad con streaming**: Aprovecha la codificación de transferencia fragmentada para archivos grandes y conexiones de larga duración
- **API flexible**: Usa HTeaPot como servidor independiente o como biblioteca en tus aplicaciones Rust

### Fácil de usar para desarrolladores
- **Configuración sencilla**: Comienza rápidamente con la configuración intuitiva de TOML
- **Diseño extensible**: Personaliza fácilmente el comportamiento para casos de uso específicos
- **Reducción de espacio**: Cero dependencias y un uso eficiente de los recursos

## Primeros pasos

### Instalación

```bash
# Instalar desde crates.io
cargo install hteapot

# O compilar desde el código fuente
git clone https://github.com/yourusername/hteapot.git
cd hteapot
cargo build --release
```

### Servidor independiente

#### Usando un archivo de configuración:

Crear un archivo `config.toml`:

```toml
[HTEAPOT]
port = 8081        # El puerto para escuchar
host = "localhost" # La dirección del host al que enlazar
root = "public"    # El directorio raíz desde el que servir los archivos
```

Run the server:

```bash
hteapot ./config.toml
```

#### Servir rápidamente un directorio:

```bash
hteapot -s ./public/
```

### Como Biblioteca

1. Añade HTeaPot a tu proyecto:

```bash
cargo add hteapot
```

2. Implementa en tu código:

```rust
use hteapot::{HttpStatus, HttpResponse, Hteapot, HttpRequest};

fn main() {
    // Crea una nueva instancia de servidor
    let server = Hteapot::new("localhost", 8081);
    
    // Define tu controlador de solicitudes
    server.listen(move |req: HttpRequest| {
        HttpResponse::new(HttpStatus::IAmATeapot, "Hello, I am HTeaPot", None)
    });
}
```

##  Rendimiento

HTeaPot se ha comparado con otros servidores HTTP populares, mostrando consistentemente excelentes métricas:

| Métrica         | HTeaPot  | Promedio de la industria |
|-----------------|----------|--------------------------|
| Solicitudes/seg | 70,000+  | 30,000-50,000            |
| Tasa de error   | <0.1%    | 0.5-2%                   |
| Latencia (p99)  | 5ms      | 15-30ms                  |
| Uso de memoria  | Low      | Moderate                 |

## Hoja de ruta (Roadmap)

- [x] Compatibilidad con HTTP/1.1 (mantenimiento activo, codificación fragmentada)
- [x] API de biblioteca
- [x] Respuestas en streaming
- [x] Gestión de formularios multiparte
- [x] Sistema de enrutamiento básico
- [ ] Compatibilidad con HTTPS
- [ ] Compresión (gzip/deflate)
- [ ] Compatibilidad con WebSockets
- [ ] Documentación y ejemplos mejorados

##  Contribuciones

¡Agradecemos las contribuciones de la comunidad! Consulta nuestro [CONTRIBUTING.md](../../CONTRIBUTING.md) para obtener las directrices sobre cómo participar.

##  Licencia

HTeaPot cuenta con la licencia MIT; consulta el archivo [LICENSE](../../LICENSE) para obtener más información.

##  Agradecimientos

- A la comunidad de Rust por sus excepcionales herramientas y bibliotecas.
- A nuestros colaboradores, que han contribuido a dar forma a este proyecto.
- A los usuarios que aportan valiosos comentarios e informes de errores.

