use super::support::{BUG, FIX, ORIGINAL, RENAMED};
use crate::budget::Budget;
use crate::claim::{self, Claim, Kind};
use crate::context::Context;
use crate::failure::{Code, Failure};
use crate::recording::Recording;
use crate::request::{self, Answer, Request};
use crate::store::Store;
use crate::subject::{Reader, Subject};
use std::collections::HashMap;

struct Memory(HashMap<&'static str, &'static str>);

impl Reader for Memory {
    fn read(&self, path: &str) -> Result<String, Failure> {
        self.0
            .get(path)
            .map(|text| (*text).to_owned())
            .ok_or_else(|| Failure::new(Code::File, format!("{path} is not in memory")))
    }
}

fn memory() -> Memory {
    Memory(HashMap::from([
        ("original.wave", ORIGINAL),
        ("renamed.wave", RENAMED),
        ("bug.wave", BUG),
        ("fix.wave", FIX),
        (
            "light.wave",
            "Light, [Light] Red, [Light] Green, [Light] Blue",
        ),
        ("block.wave", "[Boolean.Not.True] False"),
        (
            "local.wave",
            "Start, [Start] Not.True, [Not.True] False, [Not.False] True",
        ),
        (
            "and.particle",
            "[Function.And.True.True] Return.True, [Function.And.True.False] Return.False, [Function.And.False.False] Return.False",
        ),
        (
            "or.particle",
            "[Function.Or.True.True] Return.True, [Function.Or.True.False] Return.True, [Function.Or.False.False] Return.False",
        ),
        (
            "equal.particle",
            "[Function.Equal.True.True] Return.True, [Function.Equal.True.False] Return.False, [Function.Equal.False.False] Return.True",
        ),
        ("grow.wave", "Seed, [Seed] Seed.X"),
        ("left.wave", "Seed.A, [Seed] ().([A] B), [B] C"),
        ("right.wave", "[Y] Z, Root.X, [Root] ().([X] Y)"),
    ]))
}

fn file(path: &str) -> Subject {
    Subject {
        file: vec![path.to_owned()],
        ..Subject::default()
    }
}

fn recording(path: &str) -> Recording {
    Recording {
        program: Some(file(path)),
        ..Recording::default()
    }
}

fn answer(request: &Request) -> Result<Answer, Failure> {
    let reader = memory();
    let mut store = Store::default();
    request.answer(&mut Context {
        reader: &reader,
        store: &mut store,
    })
}

fn respond(text: &str, context: &mut Context<'_>) -> String {
    let serde_json::Value::Object(mut object) = serde_json::from_str(text).expect("a JSON object")
    else {
        panic!("a request is an object");
    };
    let verb = object
        .remove("verb")
        .and_then(|verb| verb.as_str().map(str::to_owned))
        .expect("a verb");
    let result = Request::read(&verb, serde_json::Value::Object(object))
        .and_then(|request| request.answer(context));
    request::envelope(&verb, &result)
}

fn session(text: &[&str]) -> Vec<serde_json::Value> {
    let reader = memory();
    let mut store = Store::default();
    let mut context = Context {
        reader: &reader,
        store: &mut store,
    };
    text.iter()
        .map(|text| serde_json::from_str(&respond(text, &mut context)).expect("an envelope"))
        .collect()
}

fn conforms(value: &serde_json::Value, schema: &serde_json::Value) -> Result<(), String> {
    use serde_json::Value;
    let branch = ["anyOf", "oneOf"]
        .into_iter()
        .filter_map(|key| schema.get(key).and_then(Value::as_array))
        .collect::<Vec<_>>();
    for option in branch {
        if !option.iter().any(|option| conforms(value, option).is_ok()) {
            return Err(format!("{value} matches no branch of {schema}"));
        }
    }
    for part in schema
        .get("allOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        conforms(value, part)?;
    }
    if let Some(kind) = schema.get("type") {
        let kind = match kind {
            Value::String(kind) => vec![kind.as_str()],
            Value::Array(kind) => kind.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        let actual = match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(number) if number.is_u64() || number.is_i64() => "integer",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        let fits = kind
            .iter()
            .any(|kind| *kind == actual || (*kind == "number" && actual == "integer"));
        if !fits {
            return Err(format!("{value} is not {kind:?}"));
        }
    }
    if let Some(allowed) = schema.get("enum").and_then(Value::as_array)
        && !allowed.contains(value)
    {
        return Err(format!("{value} is not one of {allowed:?}"));
    }
    if let Some(constant) = schema.get("const")
        && constant != value
    {
        return Err(format!("{value} is not {constant}"));
    }
    if let Value::Object(object) = value {
        let required = schema.get("required").and_then(Value::as_array);
        for key in required.into_iter().flatten().filter_map(Value::as_str) {
            if !object.contains_key(key) {
                return Err(format!("{key} is required but missing from {value}"));
            }
        }
        let property = schema.get("properties").and_then(Value::as_object);
        for (key, inner) in object {
            if let Some(schema) = property.and_then(|property| property.get(key)) {
                conforms(inner, schema)?;
            }
        }
    }
    if let (Value::Array(item), Some(schema)) = (value, schema.get("items")) {
        for item in item {
            conforms(item, schema)?;
        }
    }
    Ok(())
}

fn json(text: &str) -> serde_json::Value {
    let reader = memory();
    let mut store = Store::default();
    let response = respond(
        text,
        &mut Context {
            reader: &reader,
            store: &mut store,
        },
    );
    serde_json::from_str(&response).expect("an envelope")
}

#[test]
fn check() {
    let Ok(Answer::Check(result)) = answer(&Request::Check(crate::check::Request {
        recording: recording("bug.wave"),
        claim: vec![Claim {
            kind: Kind::Reach,
            pattern: "False.Extra".to_owned(),
            exact: true,
            preserve: true,
        }],
    })) else {
        panic!("check answers");
    };
    assert!(result.diagnostic.is_empty());
    assert_eq!(result.claim[0].answer, claim::Answer::Holds);
    assert!(
        result
            .text()
            .contains("reach False.Extra exactly   holds   s8 by e1 e7 e8")
    );
    let Ok(Answer::Check(broken)) = answer(&Request::Check(crate::check::Request {
        recording: Recording {
            program: Some(Subject {
                source: Some("A B".to_owned()),
                ..Subject::default()
            }),
            ..Recording::default()
        },
        claim: Vec::new(),
    })) else {
        panic!("check reports diagnostics as data");
    };
    assert_eq!(broken.diagnostic[0].code, "syntax");
    let location = broken.diagnostic[0].location.as_ref().expect("a location");
    assert_eq!((location.line, location.column), (1, 3));
}

#[test]
fn compare() {
    let request = |right: &str| {
        Request::Compare(crate::compare::Request {
            left: recording("original.wave"),
            right: recording(right),
            claim: Vec::new(),
            limit: 12,
        })
    };
    let Ok(Answer::Compare(bug)) = answer(&request("bug.wave")) else {
        panic!("compare answers");
    };
    assert_eq!((bug.left.configuration, bug.right.configuration), (14, 18));
    assert!(bug.configuration.lost.is_empty());
    assert!(!bug.passed());
    let mut gained = bug
        .configuration
        .gained
        .iter()
        .map(|entry| (entry.handle.as_str(), entry.text.as_str()))
        .collect::<Vec<_>>();
    gained.sort_unstable();
    assert_eq!(
        gained,
        vec![
            ("s11", "False.True.Extra"),
            ("s14", "Boolean.True.Extra"),
            ("s15", "False.Boolean.Extra"),
            ("s17", "Boolean.Boolean.Extra"),
        ]
    );
    let Ok(Answer::Compare(fix)) = answer(&request("fix.wave")) else {
        panic!("compare answers");
    };
    assert!(fix.configuration.same());
    assert!(!fix.event.same());
    assert!(!fix.passed());
    assert_eq!((fix.left.event, fix.right.event), (17, 18));
    assert_eq!(
        fix.event
            .gained
            .iter()
            .map(|group| group.rule.as_str())
            .collect::<Vec<_>>(),
        vec!["[False.Boolean] False", "[False.Boolean] False"]
    );
    assert_eq!(fix.event.lost.len(), 1);
    assert_eq!(fix.event.lost[0].rule, "[True.False] False");
    let Ok(Answer::Compare(renamed)) = answer(&request("renamed.wave")) else {
        panic!("compare answers");
    };
    assert_ne!(renamed.left.exploration, renamed.right.exploration);
    assert!(!renamed.configuration.same());
    assert!(
        renamed
            .configuration
            .gained
            .iter()
            .all(|entry| !entry.text.contains("Extra"))
    );
    let open = Recording {
        budget: Some(Budget {
            configuration: 5,
            ..Budget::default()
        }),
        ..recording("grow.wave")
    };
    let Ok(Answer::Compare(open)) = answer(&Request::Compare(crate::compare::Request {
        left: open.clone(),
        right: open,
        claim: vec![Claim {
            kind: Kind::Avoid,
            pattern: "Boom".to_owned(),
            exact: false,
            preserve: false,
        }],
        limit: 12,
    })) else {
        panic!("compare answers");
    };
    assert!(open.configuration.same() && open.event.same());
    assert_eq!(open.claim[0].left, claim::Answer::Unknown);
    assert!(!open.passed());
    assert!(
        open.text()
            .lines()
            .next()
            .is_some_and(|line| line.ends_with(" open"))
    );
}

#[test]
fn cause() {
    let value = json(r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s11.o1"}"#);
    assert_eq!(value["verb"], "cause");
    let lineage = value["answer"]["lineage"].as_array().expect("a lineage");
    let role = lineage
        .iter()
        .map(|step| step["role"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(role, vec!["remainder", "witness", "initial"]);
    assert_eq!(
        lineage[1]["text"],
        "inferred by e0 e2; the scope receives True as witness"
    );
    assert_eq!(
        lineage[0]["text"],
        "consumes False; True stays in the remainder"
    );
}

#[test]
fn select() {
    let value = json(
        r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "[False] False"}"#,
    );
    assert_eq!(value["answer"]["kind"], "event");
    assert_eq!(value["answer"]["total"], 3);
    let shorthand = json(
        r#"{"verb": "select", "program": {"source": "Seed.A, [Seed] ().([A] B)"}, "pattern": "([A] B)"}"#,
    );
    assert_eq!(shorthand["answer"]["kind"], "configuration");
    assert!(shorthand["answer"]["total"].as_u64().unwrap_or(0) > 0);
    let broken = json(r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "A B"}"#);
    assert_eq!(broken["error"]["code"], "pattern");
}

#[test]
fn inspect() {
    for (handle, kind) in [
        ("r2", "rule"),
        ("s11", "configuration"),
        ("s10.c0", "coherence"),
        ("s11.o1", "occurrence"),
        ("s10.f1", "frame"),
        ("e12", "event"),
    ] {
        let value = json(&format!(
            r#"{{"verb": "inspect", "program": {{"file": ["bug.wave"]}}, "handle": "{handle}"}}"#
        ));
        assert_eq!(value["answer"]["kind"], kind, "{handle}");
    }
    let event = json(r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "e11"}"#);
    let witness = event["answer"]["witness"]
        .as_array()
        .expect("witness occurrences")
        .iter()
        .map(|item| item["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(witness, vec!["False", "True"]);
    let produce =
        json(r#"{"verb": "inspect", "program": {"file": ["original.wave"]}, "handle": "e10"}"#);
    assert_eq!(produce["answer"]["produced"][0]["handle"], "s9.o0");
    assert_eq!(produce["answer"]["produced"][0]["text"], "False");
    let missing =
        json(r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s99"}"#);
    assert_eq!(missing["error"]["code"], "handle");
}

#[test]
fn miss() {
    let value = json(r#"{"verb": "miss", "program": {"file": ["bug.wave"]}, "rule": "r3"}"#);
    assert_eq!(value["answer"]["kind"], "rule");
    assert_eq!(value["answer"]["fired"], 0);
    let near = value["answer"]["near"].as_array().expect("near misses");
    assert!(
        near.iter()
            .all(|entry| entry["lack"][0]["missing"] == serde_json::json!(["True"]))
    );
    let target =
        json(r#"{"verb": "miss", "program": {"file": ["bug.wave"]}, "target": "Nothing.Extra"}"#);
    assert_eq!(
        target["answer"]["near"][0]["missing"],
        serde_json::json!(["Nothing"])
    );
}

#[test]
fn nearest() {
    let greedy = session(&[
        r#"{"verb": "miss", "program": {"source": "A.B, A.C"}, "target": "A, A.B"}"#,
        r#"{"verb": "miss", "program": {"source": "A.B, A.C"}, "target": "A, A.C"}"#,
        r#"{"verb": "miss", "program": {"source": "A.C, A.B"}, "target": "A.C, A"}"#,
    ]);
    for answer in &greedy {
        assert_eq!(answer["answer"]["near"][0]["handle"], "s0", "{answer}");
        assert_eq!(answer["answer"]["near"][0]["distance"], 0, "{answer}");
    }
    let exact = session(&[
        r#"{"verb": "miss", "program": {"file": ["original.wave"]}, "target": "False.Extra", "exact": true}"#,
        r#"{"verb": "miss", "program": {"file": ["original.wave"]}, "target": "False.Extra", "exact": true, "preserve": true}"#,
    ]);
    let bare = &exact[0]["answer"]["near"][0];
    assert_ne!(bare["distance"], 0, "{bare}");
    let preserved = &exact[1]["answer"]["near"][0];
    assert_eq!(preserved["handle"], "s9", "{preserved}");
    assert_eq!(preserved["distance"], 0, "{preserved}");
    let Ok(Answer::Compare(hidden)) = answer(&Request::Compare(crate::compare::Request {
        left: recording("original.wave"),
        right: recording("fix.wave"),
        claim: Vec::new(),
        limit: 0,
    })) else {
        panic!("compare answers");
    };
    assert!(hidden.event.gained.is_empty());
    assert!(hidden.event.more > 0);
    assert!(!hidden.passed());
}

#[test]
fn scope() {
    let answer = session(&[
        r#"{"verb": "check", "program": {"source": "Z, (X, [X] Y), [A] (B, C, [B, C] D)"}, "claim": [{"kind": "reach", "pattern": "Y, Z"}]}"#,
        r#"{"verb": "check", "program": {"source": "Z, (X, [X] Y)"}, "claim": [{"kind": "reach", "pattern": "Y, Z", "exact": true}]}"#,
        r#"{"verb": "check", "program": {"source": "Z, (X, [X] Y)"}, "claim": [{"kind": "reach", "pattern": "Z, (X, [X] Y)", "exact": true}]}"#,
        r#"{"verb": "miss", "program": {"source": "Z, (X, [X] Y)"}, "target": "(X, [X] Y)", "exact": true}"#,
        r#"{"verb": "select", "program": {"source": "Z, (X, [X] Y)"}, "pattern": "(X, [X] Y)"}"#,
    ]);
    assert_eq!(
        answer[0]["answer"]["claim"][0]["answer"], "holds",
        "{}",
        answer[0]
    );
    assert_eq!(
        answer[1]["answer"]["claim"][0]["answer"], "holds",
        "{}",
        answer[1]
    );
    assert_eq!(answer[2]["error"]["code"], "target", "{}", answer[2]);
    assert_eq!(answer[3]["error"]["code"], "target", "{}", answer[3]);
    assert_eq!(answer[4]["error"]["code"], "pattern", "{}", answer[4]);
}

#[test]
fn step() {
    let value = json(r#"{"verb": "step", "program": {"file": ["bug.wave"]}}"#);
    assert_eq!(value["answer"]["handle"], "s0");
    assert_eq!(value["answer"]["agenda"].as_array().map(Vec::len), Some(5));
}

#[test]
fn explore() {
    let value = json(r#"{"verb": "explore", "program": {"file": ["original.wave"]}}"#);
    let answer = &value["answer"];
    assert_eq!(answer["configuration"], 14);
    assert_eq!(answer["event"], 17);
    assert_eq!(answer["inferred"], 4);
    assert_eq!(answer["complete"], true);
    let scope = answer["end"]
        .as_array()
        .expect("end configurations")
        .iter()
        .filter(|end| end["scope"] == true)
        .count();
    assert_eq!(scope, 3);
    let key = answer["exploration"].as_str().expect("a key").to_owned();
    let reader = memory();
    let mut store = Store::default();
    let mut context = Context {
        reader: &reader,
        store: &mut store,
    };
    let first = respond(
        r#"{"verb": "explore", "program": {"file": ["original.wave"]}}"#,
        &mut context,
    );
    let again = respond(
        &format!(r#"{{"verb": "cause", "exploration": "{key}", "handle": "s12"}}"#),
        &mut context,
    );
    assert!(first.contains(&key));
    assert!(again.contains("\"path\""), "{again}");
}

#[test]
fn shape() {
    let describe = |path: &str, fix: Vec<String>| {
        let Ok(Answer::Shape(answer)) = answer(&Request::Shape(crate::shape::Request {
            program: vec![file(path)],
            target: None,
            fix,
            node: 1_000_000,
        })) else {
            panic!("shape answers");
        };
        answer.form.expect("one program has a form")
    };
    let light = describe("light.wave", Vec::new());
    assert_eq!(light.size, "6");
    assert_eq!(light.orbit.len(), 1);
    assert_eq!(light.orbit[0].len(), 3);
    assert_eq!(describe("light.wave", vec!["Red".to_owned()]).size, "2");
    assert_eq!(
        describe("block.wave", Vec::new()).block[0],
        vec!["Boolean", "Not", "True"]
    );
    let local = describe("local.wave", Vec::new());
    assert_eq!(local.size, "1");
    assert_eq!(local.pattern.len(), 1);
    let group = |path: &[&str], fix: Vec<String>| {
        let Ok(Answer::Shape(answer)) = answer(&Request::Shape(crate::shape::Request {
            program: path.iter().map(|path| file(path)).collect(),
            target: None,
            fix,
            node: 1_000_000,
        })) else {
            panic!("shape answers");
        };
        answer
    };
    let same = group(&["and.particle", "or.particle"], Vec::new());
    assert_eq!(same.class.len(), 1);
    assert_eq!(same.class[0].renaming, vec!["And → Or (True False)"]);
    assert_eq!(
        group(
            &["and.particle", "or.particle", "equal.particle"],
            Vec::new()
        )
        .class
        .len(),
        2
    );
    assert_eq!(
        group(&["and.particle", "or.particle"], vec!["True".to_owned()])
            .class
            .len(),
        2
    );
    assert_eq!(
        describe("left.wave", Vec::new()).text,
        describe("right.wave", Vec::new()).text
    );
    let Err(unknown) = answer(&Request::Shape(crate::shape::Request {
        program: vec![file("light.wave")],
        target: None,
        fix: vec!["Zed".to_owned()],
        node: 1_000_000,
    })) else {
        panic!("an atom no program names cannot be fixed");
    };
    assert_eq!(unknown.code, Code::Shape);
    assert_eq!(
        group(&["light.wave", "and.particle"], vec!["Red".to_owned()])
            .class
            .len(),
        2
    );
}

#[test]
fn protocol() {
    let request: Request = serde_json::from_value(serde_json::json!({
        "verb": "explore",
        "program": {"file": ["bug.wave"]}
    }))
    .expect("a request");
    assert_eq!(request.verb(), "explore");
    let unknown = json(r#"{"verb": "explore", "program": {"file": ["bug.wave"]}, "colour": 1}"#);
    assert_eq!(unknown["error"]["code"], "request");
    for tool in request::catalog() {
        assert_eq!(tool.input["type"], "object", "{}", tool.name);
        assert_eq!(tool.output["type"], "object", "{}", tool.name);
        assert!(!tool.description.is_empty(), "{}", tool.name);
        assert!(
            !tool.input.to_string().contains("$ref"),
            "{} inlines its schema",
            tool.name
        );
    }
}

#[test]
fn schema() {
    let request = [
        r#"{"verb": "check", "program": {"file": ["bug.wave"]}, "claim": [{"kind": "reach", "pattern": "False.Extra"}]}"#,
        r#"{"verb": "check", "program": {"source": "A B"}}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}, "mode": "path"}"#,
        r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "False"}"#,
        r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "[False] False"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "r2"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s10"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s10.c0"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s11.o1"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s10.f1"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "e11"}"#,
        r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s11"}"#,
        r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "e11"}"#,
        r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s11.o1"}"#,
        r#"{"verb": "miss", "program": {"file": ["bug.wave"]}, "target": "True.True"}"#,
        r#"{"verb": "miss", "program": {"file": ["original.wave"]}, "target": "False.Extra", "exact": true}"#,
        r#"{"verb": "miss", "program": {"file": ["bug.wave"]}, "rule": "r3"}"#,
        r#"{"verb": "step", "program": {"file": ["bug.wave"]}}"#,
        r#"{"verb": "compare", "left": {"program": {"file": ["original.wave"]}}, "right": {"program": {"file": ["bug.wave"]}}, "claim": [{"kind": "avoid", "pattern": "False.True"}]}"#,
        r#"{"verb": "shape", "program": [{"file": ["light.wave"]}]}"#,
        r#"{"verb": "shape", "program": [{"file": ["and.particle"]}, {"file": ["or.particle"]}]}"#,
    ];
    for (text, envelope) in request.iter().zip(session(&request)) {
        let verb = envelope["verb"].as_str().expect("a verb");
        let tool = request::catalog()
            .iter()
            .find(|tool| tool.name == verb)
            .expect("a tool");
        let answer = envelope
            .get("answer")
            .unwrap_or_else(|| panic!("{text}: {envelope}"));
        if let Err(failure) = conforms(answer, &tool.output) {
            panic!("{text}: {failure}");
        }
    }
}

#[test]
fn strict() {
    let answer = session(&[
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}, "colour": 1}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"], "flie": ["a.wave"]}}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}, "goal": {"configuration": "False.Extra"}}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}}"#,
    ]);
    assert_eq!(answer[0]["error"]["code"], "request");
    assert!(
        answer[0]["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("colour")
    );
    assert!(
        answer[1]["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("program.flie")
    );
    assert_eq!(answer[2]["error"]["code"], "request");
    let key = answer[3]["answer"]["exploration"]
        .as_str()
        .expect("a key")
        .to_owned();
    let reuse = session(&[
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}}"#,
        &format!(r#"{{"verb": "cause", "exploration": "{key}", "handle": "s11"}}"#),
        &format!(r#"{{"verb": "cause", "exploration": "{key}", "mode": "path", "handle": "s11"}}"#),
        &format!(
            r#"{{"verb": "check", "exploration": "{key}", "claim": [{{"kind": "reach", "pattern": "False.True"}}]}}"#
        ),
    ]);
    assert_eq!(reuse[1]["answer"]["exploration"], key);
    assert_eq!(reuse[2]["error"]["code"], "request");
    assert_eq!(reuse[3]["answer"]["claim"][0]["answer"], "holds");
}

#[test]
fn primer() {
    let grammar = include_str!("../../frontend/parser.rs");
    let block = crate::resource::find("photonic://primer")
        .expect("the primer is a resource")
        .text
        .split("```")
        .nth(1)
        .expect("the primer shows the grammar");
    for line in block.lines().filter(|line| !line.trim().is_empty()) {
        assert!(grammar.contains(line), "the grammar has no line {line}");
    }
}

#[test]
fn pattern() {
    let case: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../book/pattern.json")).expect("the shared cases");
    for case in &case {
        let request = serde_json::json!({
            "verb": "select",
            "program": {"source": case["program"]},
            "pattern": case["pattern"],
        });
        let value = json(&request.to_string());
        let label = format!("{} in {}", case["pattern"], case["program"]);
        if let Some(code) = case.get("error") {
            assert_eq!(&value["error"]["code"], code, "{label}");
            continue;
        }
        assert_eq!(value["answer"]["kind"], case["kind"], "{label}");
        assert_eq!(value["answer"]["total"], case["total"], "{label}");
    }
}

#[test]
fn fixed() {
    let request = |fix: &[&str]| {
        Request::Shape(crate::shape::Request {
            program: ["[Y.Q] R", "[X.Q] R"]
                .map(|source| Subject {
                    source: Some(source.to_owned()),
                    ..Subject::default()
                })
                .to_vec(),
            target: None,
            fix: fix.iter().map(|&name| name.to_owned()).collect(),
            node: crate::shape::NODE,
        })
    };
    let Ok(Answer::Shape(free)) = answer(&request(&[])) else {
        panic!("shape answers");
    };
    assert_eq!(free.class.len(), 1);
    let Ok(Answer::Shape(fixed)) = answer(&request(&["X", "Y"])) else {
        panic!("shape answers");
    };
    assert_eq!(fixed.class.len(), 2);
}

#[test]
fn placement() {
    let crowded =
        json(r#"{"verb": "select", "program": {"source": "A.B, A"}, "pattern": "A, A.B"}"#);
    assert_eq!(crowded["answer"]["total"], 1);
    let source = vec!["A"; 16].join(", ");
    let pattern = format!("{}, B", vec!["A"; 15].join(", "));
    let wide = json(
        &serde_json::json!({"verb": "select", "program": {"source": source}, "pattern": pattern})
            .to_string(),
    );
    assert_eq!(wide["answer"]["total"], 0);
}
