# HteaPot HTTP Server
Spanish | [English](../readme.md)

Hteapot es un potente servidor HTTP y biblioteca escrita en Rust, diseñada para aplicaciones web de alto rendimiento. Ofrece una forma sencilla y eficiente de servir archivos estáticos y gestionar solicitudes HTTP con gran rapidez y resiliencia.

# Funcionalidades

### 1. **Arquitectura basada en Hilos**
   - Sistema personalizado de hilos, capaz de manejar aproximadamente **70,000 solicitudes por segundo**.
   - Prioriza la resistencia sobre la velocidad máxima, haciéndolo robusto bajo cargas pesadas.

### 2. **Rendimiento Bajo Carga**
   - Desempeño estable con alta concurrencia, gestionando hasta **50,000 solicitudes por segundo** con conexiones aumentadas.
   - Mientras que el rendimiento de otros servidores se degrada bajo alta carga, Hteapot se mantiene estable.

### 3. **Baja Tasa de Errores**
   - Logra una tasa de éxito cercana al **100% de respuestas 200 OK** en pruebas de estrés, demostrando su gran resistencia.
   - Supera a otros servidores en condiciones similares, con una tasa mínima de errores en concurrencias extremas.

# Uso

## Servidor HTTP independiente

Puedes configurar el servidor utilizando un archivo TOML. Aquí tienes un ejemplo de configuración:

```toml
[HTEAPOT]
port = 8081 # Puerto en el que el servidor escuchará las conexiones entrantes.
host = "localhost" # Dirección de host en la que se enlazará el servidor.
root = "public" # Directorio raíz desde el cual se servirán los archivos.
