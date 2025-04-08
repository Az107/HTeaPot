# Configuration

HTeaPot can be configured via a **TOML** configuration file. Below is an example configuration and explanation of each option available.

---

## Example Configuration

```toml
[HTEAPOT]
port = 8081                # Port number to listen
host = "0.0.0.0"           # Host name or IP address
threads = 4                # Number of threads in the thread pool
root = "public"            # Root directory to serve files
cache = true               # Enable or disable cache
cache_ttl = 36             # Cache Time-To-Live in seconds
log_file = "path/to/log"   # Log file path (remove to print to stdout)
index = "index.html"      # Default file to serve for root

[proxy]
"/test" = "http://example.com"
"/google" = "http://google.com"
```

## Configuration Options


### **[hteapot]**
- **port**: (Type: u16) The port number on which the server will listen for incoming requests.
  Default: 8081

- **host**: (Type: String) The host name or IP address that the server will bind to.
Default: "0.0.0.0" (listens on all available interfaces)

- **root**: (Type: String) The root directory from which files will be served. This is the location where your static files (HTML, CSS, JS) will be stored.
Default: "public"

- **cache**: (Type: bool) Whether to enable or disable caching of served files. Enabling the cache can improve performance, but may serve outdated files.
Default: false

- **cache_ttl**: (Type: u16) Time-to-Live (TTL) for cached files in seconds. Defines how long the server will cache a file before checking for updates.
Default: 36

- **threads**:
(Type: u16) Number of threads to use in the thread pool for handling requests. More threads can improve performance in multi-core systems.
Default: 4

- **log_file**: (Type: Option<String>) The path to the log file where logs will be written. If None, logs will be printed to stdout.
Default: None

- **index**:
(Type: String) The default file to serve when accessing the root URL (/).
Default: "index.html"

### **[proxy]**
A list of reverse proxy rules. These rules allow requests to certain paths to be forwarded to a different URL. For example, you can proxy /google to http://google.com.
Example:

```TOML
[proxy]
"/test" = "http://example.com"
"/google" = "http://google.com"
```
The rules are applied in order, so if a request matches multiple paths, the first matching rule will be used.
**Note:** if the rot path (/) is defined in proxy, it will override all other paths and forward all requests to the specified URL.


## How to Load Configuration
Just pases the config path to the cli as args
```bash
hteapot <config file>
```
