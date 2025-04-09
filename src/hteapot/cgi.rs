use std::collections::HashMap;
use std::process::{Command, Output};
use std::io;
use std::io::Write;

/// Executes a CGI script with the given environment variables, arguments, and optional input.
/// - `script_path`: Path to the CGI script to execute.
/// - `env_vars`: A `HashMap` of environment variables to set for the script.
/// - `args`: A vector of arguments to pass to the script.
/// - `input`: Optional input data to provide to the script's standard input.
/// - `io::Result<Output>`: The output of the executed script, including stdout, stderr, and exit status.
pub fn execute_cgi(script_path: &str, env_vars: HashMap<String, String>, args: Vec<String>, input: Option<&[u8]>) -> io::Result<Output> {
    let mut command = Command::new(script_path);

    for (key, value) in env_vars {
        command.env(key, value);
    }

    if !args.is_empty() {
        command.args(&args);
    }

    if let Some(input_data) = input {
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command.spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input_data)?;
        }

        child.wait_with_output()
    } else {
        command.output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Test case to verify successful execution of a CGI script.
    /// This test uses a platform-specific command (`cmd.exe` on Windows, `sh` on Unix)
    /// to simulate a CGI script that outputs "Hello, World!".
    #[test]
    fn test_execute_cgi_success() {
        // Determine the script path and arguments based on the operating system
        let script_path = if cfg!(windows) { "cmd.exe" } else { "sh" };
        let mut env_vars = HashMap::new(); // Environment variables (empty in this test)
        let args = if cfg!(windows) {
            vec!["/C".to_string(), "echo".to_string(), "Hello, World!".to_string()]
        } else {
            vec!["-c".to_string(), "echo Hello, World!".to_string()]
        };

        let input = None; 

        let result = execute_cgi(script_path, env_vars, args, input);
        assert!(result.is_ok()); 
        let output = result.unwrap();
        assert!(output.status.success()); 
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello"), "Expected 'Hello' in output, got: {}", stdout);
    }

    /// Test case to verify failure when attempting to execute a non-existent CGI script.
    #[test]
    fn test_execute_cgi_failure() {
        let script_path = "non_existent_script.cgi"; 
        let env_vars = HashMap::new(); 
        let args = vec![]; // No arguments
        let input = None; // No input data

        let result = execute_cgi(script_path, env_vars, args, input);
        assert!(result.is_err()); // Ensure the execution failed
    }
}