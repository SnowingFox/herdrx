//! Running programs for the app and its mods: here, or on another machine
//! through the core's exec, bounded in time and output.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

pub fn shell() -> String {
    if cfg!(windows) {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let pwsh = std::env::split_paths(&path).any(|d| d.join("pwsh.exe").is_file());
        return if pwsh { "pwsh.exe" } else { "powershell.exe" }.into();
    }
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
}

const EXEC_LIMIT: Duration = Duration::from_secs(30);
const EXEC_OUTPUT: u64 = 8 << 20;

pub fn run_blocking(command: &str, args: &[String], cwd: Option<String>) -> Result<(i32, String, String), String> {
    use std::io::Read;
    let mut child = crate::spawn::command(command)
        .args(args)
        .current_dir(cwd.map(PathBuf::from).unwrap_or_else(crate::paths::home))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {command}: {e}"))?;
    // Bounded: a chatty command can't fill memory; the rest is drained and dropped.
    let drain = |r: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut r = r;
            let _ = (&mut r).take(EXEC_OUTPUT).read_to_end(&mut buf);
            let _ = std::io::copy(&mut r, &mut std::io::sink());
            String::from_utf8_lossy(&buf).into_owned()
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > EXEC_LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{command} took longer than 30 s and was stopped"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(e.to_string()),
        }
    };
    Ok((status.code().unwrap_or(-1), out.join().unwrap_or_default(), err.join().unwrap_or_default()))
}

/// Runs a program on another machine through the core's exec: its exact output.
pub fn run_remote(machine: &str, command: &str, args: &[String], cwd: Option<String>) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    use base64::Engine;
    let mut hc_args: Vec<String> = vec!["exec".into(), "--json".into(), "--on".into(), machine.into(), "--timeout".into(), EXEC_LIMIT.as_secs().to_string()];
    if let Some(c) = cwd {
        hc_args.extend(["--cwd".into(), c]);
    }
    hc_args.push("--".into());
    hc_args.push(command.into());
    hc_args.extend(args.iter().cloned());
    let (_, out, err) = run_blocking("hc", &hc_args, None)?;
    let v: Value = serde_json::from_str(out.trim()).or_else(|_| serde_json::from_str(err.trim())).map_err(|_| format!("hc exec: {}", err.trim()))?;
    if let Some(e) = v.get("error") {
        return Err(e.get("message").and_then(Value::as_str).unwrap_or("hc exec failed").to_owned());
    }
    if v.get("timed_out").and_then(Value::as_bool) == Some(true) {
        return Err(format!("{command} took longer than {} s on {machine} and was stopped", EXEC_LIMIT.as_secs()));
    }
    let bytes = |k: &str| base64::engine::general_purpose::STANDARD.decode(v.get(k).and_then(Value::as_str).unwrap_or_default()).map_err(|e| e.to_string());
    let code = v.get("code").and_then(Value::as_i64).map_or(-1, |c| c as i32);
    Ok((code, bytes("stdout")?, bytes("stderr")?))
}

/// A program's output here, if it ran (for the app's own checks).
pub fn run_here(command: &str, args: &[String]) -> Option<String> {
    run_blocking(command, args, None).ok().map(|(_, out, _)| out)
}

/// A program's output on another machine, if it ran.
pub fn run_on(machine: &str, command: &str, args: &[String]) -> Option<String> {
    run_remote(machine, command, args, None).ok().map(|(_, out, _)| String::from_utf8_lossy(&out).into_owned())
}

