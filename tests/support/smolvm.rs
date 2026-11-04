use serde_json::{json, Value};
use std::{collections::BTreeSet, env, fs, io::Write, path::PathBuf, process};

fn var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}
fn enabled(name: &str) -> bool {
    var(name) == "1"
}
fn fail(code: i32, message: &str) -> ! {
    eprintln!("{message}");
    process::exit(code)
}
fn option(args: &[String], flag: &str) -> Option<String> {
    args.iter().enumerate().find_map(|(i, arg)| {
        if arg == flag {
            args.get(i + 1).cloned()
        } else {
            arg.strip_prefix(&format!("{flag}=")).map(str::to_owned)
        }
    })
}
fn options(args: &[String], flag: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter_map(|(i, arg)| {
            if arg == flag {
                args.get(i + 1).cloned()
            } else {
                arg.strip_prefix(&format!("{flag}=")).map(str::to_owned)
            }
        })
        .collect()
}
fn inventory_path() -> Option<PathBuf> {
    env::var_os("SMOLVM_TEST_INVENTORY_FILE")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}
fn inventory() -> Value {
    let raw = if let Some(path) = inventory_path() {
        fs::read(path).unwrap()
    } else if !var("SMOLVM_TEST_INVENTORY").is_empty() {
        var("SMOLVM_TEST_INVENTORY").into_bytes()
    } else {
        return Value::Array(
            var("SMOLVM_TEST_MACHINES")
                .lines()
                .filter(|s| !s.is_empty())
                .map(|name| json!({"name":name,"state":"stopped","labels":{}}))
                .collect(),
        );
    };
    serde_json::from_slice(&raw).unwrap_or_else(|_| fail(70, "invalid test inventory"))
}
fn save(value: &Value) {
    if let Some(path) = inventory_path() {
        let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap()).unwrap();
        serde_json::to_writer(&mut temporary, value).unwrap();
        temporary.persist(path).unwrap();
    }
}
fn gate(stage: &str) {
    let Some(ready) = env::var_os(format!("SMOLVM_TEST_{stage}_GATE_READY")) else {
        return;
    };
    let release = env::var_os(format!("SMOLVM_TEST_{stage}_GATE_RELEASE"))
        .map(PathBuf::from)
        .expect("a gate requires a release path");
    fs::write(ready, "ready").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !release.exists() {
        if std::time::Instant::now() >= deadline {
            fail(76, "test coordination gate timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn runtime_output(stage: &str) {
    if enabled("SMOLVM_TEST_OUTPUT") {
        println!("runtime-{stage}-stdout");
        eprintln!("runtime-{stage}-stderr");
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
    }
    if var("SMOLVM_TEST_LARGE_OUTPUT_STAGE") == stage {
        let stdout = std::thread::spawn(|| {
            let mut stream = std::io::stdout().lock();
            for _ in 0..256 {
                stream.write_all(&[b'O'; 4096]).unwrap();
            }
            stream.write_all(b"\nlarge-stdout-finished\n").unwrap();
            stream.flush().unwrap();
        });
        let stderr = std::thread::spawn(|| {
            let mut stream = std::io::stderr().lock();
            for _ in 0..256 {
                stream.write_all(&[b'E'; 4096]).unwrap();
            }
            stream.write_all(b"\nlarge-stderr-finished\n").unwrap();
            stream.flush().unwrap();
        });
        stdout.join().unwrap();
        stderr.join().unwrap();
    }
    if var("SMOLVM_TEST_OUTPUT_GATE_STAGE") == stage {
        gate("OUTPUT");
    }
}
#[cfg(target_os = "linux")]
fn terminal_dimensions() {
    #[repr(C)]
    #[derive(Default)]
    struct Dimensions {
        rows: u16,
        columns: u16,
        x_pixels: u16,
        y_pixels: u16,
    }
    unsafe extern "C" {
        fn ioctl(fd: i32, request: std::os::raw::c_ulong, dimensions: *mut Dimensions) -> i32;
    }
    let mut dimensions = Dimensions::default();
    if unsafe { ioctl(0, 0x5413, &mut dimensions) } == 0 {
        println!("guest-size:{}x{}", dimensions.rows, dimensions.columns);
    }
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(log) = env::var_os("SMOLVM_TEST_LOG") {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .unwrap();
        file.write_all(format!("{}\n", args.join(" ")).as_bytes())
            .unwrap();
    }
    if let Some(log) = env::var_os("SMOLVM_TEST_CALLER_LOG") {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .unwrap();
        let mut record =
            serde_json::to_vec(&json!({"caller":var("SMOLVM_TEST_CALLER"),"args":args})).unwrap();
        record.push(b'\n');
        file.write_all(&record).unwrap();
    }
    if let Some(log) = env::var_os("SMOLVM_TEST_ARGV_LOG") {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .unwrap();
        let mut record = serde_json::to_vec(&args).unwrap();
        record.push(b'\n');
        file.write_all(&record).unwrap();
    }
    if args == ["--version"] {
        if enabled("SMOLVM_TEST_OUTPUT") {
            eprintln!("query-version-stderr");
        }
        if enabled("SMOLVM_TEST_VERSION_FAIL") {
            fail(69, "simulated version failure");
        }
        gate("VERSION");
        println!(
            "{}",
            env::var("SMOLVM_TEST_VERSION").unwrap_or("smolvm test 1.19.2".into())
        );
        return;
    }
    if args.iter().any(|s| s == "--help") && !args.iter().any(|s| s == "--") {
        if enabled("SMOLVM_TEST_OUTPUT") {
            eprintln!("query-help-stderr");
        }
        if enabled("SMOLVM_TEST_HELP_FAIL") {
            fail(68, "simulated capability failure");
        }
        if args.get(1).map(String::as_str) == Some("update")
            && enabled("SMOLVM_TEST_INCOMPATIBLE_UPDATE")
        {
            println!("Usage: smolvm machine update --name NAME");
        } else {
            println!("SmolVM test adapter: machine ls create start stop delete update exec; pack create; --json --name --force --mem --cpus --smolfile --from --from-vm --label --volume --allow-host --port --remove-port --secret-env -it -o");
        }
        return;
    }
    if args.get(..3) == Some(&["machine".into(), "ls".into(), "--json".into()]) {
        if enabled("SMOLVM_TEST_OUTPUT") {
            eprintln!("query-inventory-stderr");
        }
        let sequence = var("SMOLVM_TEST_INVENTORY_SEQUENCE_FILE");
        let call = if !sequence.is_empty() {
            let count = env::var("SMOLVM_TEST_INVENTORY_COUNT_FILE")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or(format!("{sequence}.count"));
            let n = fs::read_to_string(&count)
                .unwrap_or_default()
                .trim()
                .parse::<usize>()
                .unwrap_or(0)
                + 1;
            fs::write(count, n.to_string()).unwrap();
            n
        } else {
            1
        };
        if enabled("SMOLVM_TEST_INVENTORY_FAIL")
            || var("SMOLVM_TEST_INVENTORY_FAIL_AT") == call.to_string()
        {
            fail(70, "simulated inventory failure");
        }
        let mut value = if sequence.is_empty() {
            inventory()
        } else {
            let raw = fs::read_to_string(sequence).unwrap();
            let line = raw
                .lines()
                .nth(call - 1)
                .unwrap_or_else(|| fail(70, "missing simulated inventory response"));
            match serde_json::from_str::<Value>(line) {
                Ok(v) => v,
                Err(_) => {
                    println!("{line}");
                    return;
                }
            }
        };
        if let Some(machines) = value.as_array_mut() {
            for machine in machines {
                if let Some(obj) = machine.as_object_mut() {
                    if let Some(ports) = obj.remove("_test_ports") {
                        obj.insert("ports".into(), json!(ports.as_array().unwrap().len()));
                    }
                    obj.entry("ports").or_insert(json!(0));
                    obj.entry("created_at").or_insert(json!(1));
                }
            }
        }
        println!("{value}");
        return;
    }
    if args.first().map(String::as_str) == Some("pack")
        && args.get(1).map(String::as_str) == Some("create")
    {
        runtime_output("pack");
        if enabled("SMOLVM_TEST_PACK_FAIL") {
            fail(74, "simulated pack failure");
        }
        let output = option(&args, "-o").unwrap();
        fs::write(
            format!("{output}.smolmachine"),
            if enabled("SMOLVM_TEST_EMPTY_PACK") {
                ""
            } else {
                "artifact\n"
            },
        )
        .unwrap();
        return;
    }
    if args.first().map(String::as_str) != Some("machine") {
        fail(64, "unexpected test adapter command");
    }
    let operation = args.get(1).map(String::as_str).unwrap_or("");
    let name = option(&args, "--name").unwrap_or_default();
    let interactive = operation == "exec" && args.iter().any(|arg| arg == "-it");
    if !interactive {
        runtime_output(if operation == "exec" {
            "seed"
        } else {
            operation
        });
    }
    if operation == "update" {
        gate("UPDATE");
    }

    for (op, flag, code) in [
        ("create", "CREATE", 71),
        ("start", "START", 75),
        ("stop", "STOP", 72),
        ("delete", "DELETE", 73),
        ("update", "UPDATE", 75),
    ] {
        if operation == op
            && (enabled(&format!("SMOLVM_TEST_{flag}_FAIL"))
                || (!name.is_empty() && var(&format!("SMOLVM_TEST_{flag}_FAIL_NAME")) == name))
        {
            fail(code, &format!("simulated machine {op} failure"));
        }
    }
    if operation == "exec" {
        if args
            .iter()
            .any(|s| s.contains("/opt/") && s.contains("cp "))
        {
            if let Ok(ms) = var("SMOLVM_TEST_SEED_SLEEP_MS").parse::<u64>() {
                if let Some(path) = env::var_os("SMOLVM_TEST_SEED_READY") {
                    fs::write(path, "ready").unwrap();
                }
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            if enabled("SMOLVM_TEST_SEED_FAIL") {
                fail(1, "simulated seed failure");
            }
            return;
        }
        if enabled("SMOLVM_TEST_OUTPUT") {
            println!("guest-stdout");
            eprintln!("guest-stderr");
            std::io::stdout().flush().unwrap();
        }
        if enabled("SMOLVM_TEST_STDIN_ROUNDTRIP") {
            use std::io::{BufRead, IsTerminal};
            println!(
                "guest-terminals: stdin={} stdout={} stderr={}",
                std::io::stdin().is_terminal(),
                std::io::stdout().is_terminal(),
                std::io::stderr().is_terminal()
            );
            #[cfg(target_os = "linux")]
            terminal_dimensions();
            println!("guest-prompt>\x1b[32mready\x1b[0m");
            std::io::stdout().flush().unwrap();
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line).unwrap();
            #[cfg(target_os = "linux")]
            terminal_dimensions();
            print!("guest-received:{line}");
            std::io::stdout().flush().unwrap();
        }
        if let Ok(ms) = var("SMOLVM_TEST_EXEC_SLEEP_MS").parse::<u64>() {
            if let Some(path) = env::var_os("SMOLVM_TEST_EXEC_READY") {
                fs::write(path, "ready").unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
        process::exit(var("SMOLVM_TEST_EXEC_STATUS").parse().unwrap_or(0));
    }
    if operation == "status" {
        return;
    }
    let mut value = inventory();
    let machines = value
        .as_array_mut()
        .unwrap_or_else(|| fail(70, "test inventory must be array"));
    match operation {
        "create" => {
            if machines.iter().any(|m| m["name"] == name) {
                fail(71, "machine already exists");
            }
            let labels: serde_json::Map<String, Value> = options(&args, "--label")
                .iter()
                .map(|label| {
                    let (key, value) = label.split_once('=').unwrap();
                    (key.into(), json!(value))
                })
                .collect();
            machines.push(json!({"name":name,"state":"created","labels":labels,"ports":0,"_test_ports":[],"created_at":var("SMOLVM_TEST_CREATED_AT").parse::<u64>().unwrap_or(1)}));
        }
        "start" | "stop" => {
            for m in machines {
                if m["name"] == name {
                    m["state"] = json!(if operation == "start" {
                        "running"
                    } else {
                        "stopped"
                    });
                }
            }
        }
        "delete" => machines.retain(|m| m["name"] != name),
        "update" => {
            let m = machines
                .iter_mut()
                .find(|m| m["name"] == name)
                .unwrap_or_else(|| fail(75, "missing update target"));
            if !matches!(m["state"].as_str(), Some("stopped" | "created")) {
                fail(75, "update target is running");
            }
            let mut ports: BTreeSet<u64> = m["_test_ports"]
                .as_array()
                .map(|a| a.iter().map(|p| p.as_u64().unwrap()).collect())
                .unwrap_or_default();
            for p in options(&args, "--remove-port") {
                ports.remove(&p.split(':').next().unwrap().parse::<u64>().unwrap());
            }
            for p in options(&args, "--port") {
                ports.insert(p.split(':').next().unwrap().parse::<u64>().unwrap());
            }
            if enabled("SMOLVM_TEST_UPDATE_WRONG_COUNT") {
                ports.insert(65535);
            }
            m["ports"] = json!(ports.len());
            m["_test_ports"] = json!(ports);
        }
        _ => fail(64, "unexpected machine operation"),
    }
    save(&value);
    if operation == "create" && enabled("SMOLVM_TEST_CREATE_FAIL_AFTER") {
        fail(71, "simulated failure after machine registration");
    }
    if operation == "start" {
        if let Ok(ms) = var("SMOLVM_TEST_START_SLEEP_MS").parse::<u64>() {
            if let Some(path) = env::var_os("SMOLVM_TEST_START_READY") {
                fs::write(path, "ready").unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }
    if operation == "update" && enabled("SMOLVM_TEST_UPDATE_FAIL_AFTER") {
        fail(75, "simulated interruption after update");
    }
}
