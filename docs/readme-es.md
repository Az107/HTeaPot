# HteaPot HTTP Server
Spanish | [English](README.md)

HteaPot es un servidor HTTP simple escrito en Rust. Te permite servir archivos estáticos y manejar solicitudes HTTP básicas.

# Características

 - Servir archivos estáticos desde un directorio raíz especificado
 - Puerto y host del servidor configurables
 - Registro básico de solicitudes entrantes

# Uso

1. Clonar el repositorio:
```bash
git clone <repository_url>
```

2. Compilar el proyecto:
```bash
cargo build --release
```
Ejecutar el servidor con un archivo de configuración:
```bash
Copy code
./target/release/hteapot <config_file_path>
```
# Configuración

Puedes configurar el servidor usando un archivo TOML. Aquí tienes un ejemplo de configuración:

```toml
[HTEAPOT]
port = 8081 # The port on which the server will listen for incoming connections.
host = "localhost" # The host address to bind the server to. 
root = "public" # The root directory from which to serve files.
```
# Contribuciones

¡Las contribuciones son bienvenidas! Siéntete libre de abrir problemas o enviar solicitudes de extracción.

# Licencia

Este proyecto está licenciado bajo la Licencia MIT - consulta el archivo LICENSE para más detalles.