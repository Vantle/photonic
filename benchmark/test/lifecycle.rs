use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn execute(name: &str, argument: &[&str]) -> Output {
    let binary = std::env::var_os(name).unwrap();
    let binary = runfiles::Runfiles::create()
        .unwrap()
        .rlocation_from(binary, "")
        .unwrap();
    Command::new(binary).args(argument).output().unwrap()
}

fn report(name: &str, argument: &[&str]) -> Value {
    let output = execute(name, argument);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn verify(argument: &[&str]) {
    let plain = report("LIFECYCLE", argument);
    let counted = report("ALLOCATION", argument);
    assert_eq!(plain["allocation"], false);
    assert_eq!(counted["allocation"], true);
    assert_eq!(plain["measurement"].as_array().unwrap().len(), 2);
    assert_eq!(counted["measurement"].as_array().unwrap().len(), 2);
    for (plain, counted) in std::iter::once((&plain["cold"], &counted["cold"])).chain(
        plain["measurement"]
            .as_array()
            .unwrap()
            .iter()
            .zip(counted["measurement"].as_array().unwrap()),
    ) {
        assert_eq!(plain["observation"], counted["observation"]);
        assert!(plain["observation"]["byte"].as_u64().unwrap() > 0);
        let mut allocated = 0;
        let mut released = 0;
        let mut retained = 0;
        let mut peak = 0;
        for phase in [
            "initialization",
            "execution",
            "reporting",
            "serialization",
            "release",
        ] {
            assert!(plain[phase].get("allocation").is_none());
            assert!(plain[phase]["duration"].as_f64().unwrap() >= 0.0);
            let allocation = &counted[phase]["allocation"];
            allocated += allocation["allocated"].as_i64().unwrap();
            released += allocation["released"].as_i64().unwrap();
            peak = peak.max(retained + allocation["peak"].as_i64().unwrap());
            retained += allocation["retained"].as_i64().unwrap();
            assert_eq!(allocated - released, retained);
            assert!(retained >= 0);
            assert!(peak >= retained);
        }
        assert_eq!(retained, 0);
        assert_eq!(counted["footprint"]["allocated"], allocated);
        assert_eq!(counted["footprint"]["released"], released);
        assert_eq!(counted["footprint"]["retained"], retained);
        assert_eq!(counted["footprint"]["peak"], peak);
        assert_eq!(
            counted["retention"]
                .as_object()
                .unwrap()
                .values()
                .map(|value| value.as_i64().unwrap())
                .sum::<i64>(),
            -counted["release"]["allocation"]["retained"]
                .as_i64()
                .unwrap()
        );
    }
}

#[test]
fn lifecycle() {
    let directory = PathBuf::from(std::env::var_os("TEST_TMPDIR").unwrap());
    let path = directory.join("lifecycle.wave");
    std::fs::write(&path, "A [A] B [B] C").unwrap();
    let source = path.to_str().unwrap();
    verify(&["direct", source, "C", "--sample", "2"]);
    verify(&["exhaustive", source, "--sample", "2"]);
    verify(&["expression", "2+2", "11", "--sample", "2"]);
    for name in ["LIFECYCLE", "ALLOCATION"] {
        for argument in [
            vec!["direct", source, "C", "--sample", "0"],
            vec!["direct", source, "C", "--budget", "0"],
            vec!["direct", source, "C", "--budget", "1"],
            vec!["direct", source, "Missing"],
            vec!["expression", "3+1", "11"],
            vec!["expression", "2+2", "3"],
        ] {
            assert!(!execute(name, &argument).status.success());
        }
    }
}

#[test]
fn export() {
    let directory = PathBuf::from(std::env::var_os("TEST_TMPDIR").unwrap());
    let path = directory.join("export.wave");
    std::fs::write(&path, "A [A] B [B] C").unwrap();
    let source = path.to_str().unwrap();
    for case in [vec!["direct", source, "C"], vec!["exhaustive", source]] {
        let mut expected = None;
        for name in ["LIFECYCLE", "ALLOCATION"] {
            for mode in ["owned", "view"] {
                for writer in [false, true] {
                    let mut argument = case.clone();
                    argument.extend(["--export", mode, "--sample", "2"]);
                    if writer {
                        argument.push("--writer");
                    }
                    let value = report(name, &argument);
                    for value in std::iter::once(&value["cold"])
                        .chain(value["measurement"].as_array().unwrap())
                    {
                        assert_eq!(value["mode"], mode);
                        assert_eq!(value["writer"], writer);
                        let observation = (value["byte"].clone(), value["fingerprint"].clone());
                        assert!(observation.0.as_u64().unwrap() > 0);
                        assert_eq!(expected.get_or_insert(observation.clone()), &observation);
                        if name == "ALLOCATION" {
                            let mut retained = 0;
                            let mut peak = 0;
                            let mut allocated = 0;
                            let mut released = 0;
                            for phase in ["initialization", "execution", "export", "release"] {
                                let allocation = &value[phase]["allocation"];
                                peak = peak.max(retained + allocation["peak"].as_i64().unwrap());
                                retained += allocation["retained"].as_i64().unwrap();
                                allocated += allocation["allocated"].as_u64().unwrap();
                                released += allocation["released"].as_u64().unwrap();
                            }
                            assert_eq!(retained, 0);
                            assert_eq!(value["footprint"]["retained"], 0);
                            assert_eq!(value["footprint"]["peak"], peak);
                            assert_eq!(value["footprint"]["allocated"], allocated);
                            assert_eq!(value["footprint"]["released"], released);
                            assert_eq!(allocated, released);
                        }
                    }
                }
            }
        }
    }
}
