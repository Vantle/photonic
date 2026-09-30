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
        ("fired.wave", "X, (A, [A] B)"),
        ("rule.particle", "[A] B"),
        ("seed.wave", "A"),
        ("idle.wave", "X, (B, [A] B)"),
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

fn source(text: &str) -> Recording {
    Recording {
        program: Some(Subject {
            source: Some(text.to_owned()),
            ..Subject::default()
        }),
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
            .contains("reach False.Extra exactly   holds   s13 by e0 e7 e19")
    );
    let Ok(Answer::Check(broken)) = answer(&Request::Check(crate::check::Request {
        recording: Recording {
            program: Some(Subject {
                source: Some("A B)".to_owned()),
                ..Subject::default()
            }),
            ..Recording::default()
        },
        claim: Vec::new(),
    })) else {
        panic!("check reports diagnostics as data");
    };
    assert_eq!(broken.diagnostic[0].code, Code::Source);
    assert_eq!(broken.diagnostic[0].diagnostic.as_deref(), Some("syntax"));
    let location = broken.diagnostic[0].location.as_ref().expect("a location");
    assert_eq!((location.line, location.column), (1, 4));
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
    assert!(!bug.passed());
    let scoped = |entry: &crate::compare::Entry| entry.text.starts_with("in f1:");
    let root = bug
        .configuration
        .gained
        .iter()
        .filter(|entry| !scoped(entry))
        .map(|entry| (entry.handle.as_str(), entry.text.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        root,
        std::collections::BTreeSet::from([
            ("s14", "False.Boolean.Extra"),
            ("s4", "Boolean.True.Extra"),
            ("s6", "False.True.Extra"),
            ("s7", "Boolean.Boolean.Extra"),
        ])
    );
    assert!(bug.configuration.lost.iter().all(scoped));
    let Ok(Answer::Compare(fix)) = answer(&request("fix.wave")) else {
        panic!("compare answers");
    };
    assert_eq!((fix.left.configuration, fix.right.configuration), (14, 14));
    assert!(!fix.configuration.same());
    assert!(
        fix.configuration
            .gained
            .iter()
            .chain(&fix.configuration.lost)
            .all(scoped)
    );
    assert!(!fix.passed());
    assert_eq!((fix.left.event, fix.right.event), (17, 18));
    let rule = |group: &[crate::compare::Group]| {
        group
            .iter()
            .map(|group| group.rule.clone())
            .collect::<Vec<_>>()
    };
    assert!(rule(&fix.event.gained).contains(&"[False.Boolean] False".to_owned()));
    assert!(rule(&fix.event.lost).contains(&"[True.False] False".to_owned()));
    let Ok(Answer::Compare(edited)) = answer(&Request::Compare(crate::compare::Request {
        left: source("A, [A] B, [B] C"),
        right: source("A, [A] B, [A] C"),
        claim: Vec::new(),
        limit: 12,
    })) else {
        panic!("compare answers");
    };
    assert!(edited.configuration.same(), "{}", edited.text());
    assert!(!edited.event.same());
    assert_eq!(rule(&edited.event.gained), vec!["[A] C"]);
    assert!(
        rule(&edited.event.lost).iter().all(|rule| rule == "[B] C"),
        "{}",
        edited.text()
    );
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
    let Ok(Answer::Compare(scoped)) = answer(&Request::Compare(crate::compare::Request {
        left: recording("fired.wave"),
        right: recording("idle.wave"),
        claim: Vec::new(),
        limit: 12,
    })) else {
        panic!("compare answers");
    };
    assert_ne!(scoped.left.exploration, scoped.right.exploration);
    assert_eq!((scoped.left.event, scoped.right.event), (1, 0));
    let open = Recording {
        budget: Some(Budget {
            configuration: Some(5),
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
    let value = json(r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s6.o1"}"#);
    assert_eq!(value["verb"], "cause");
    let lineage = value["answer"]["lineage"].as_array().expect("a lineage");
    let role = lineage
        .iter()
        .map(|step| step["role"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(role, vec!["remainder", "witness", "initial"]);
    assert_eq!(
        lineage[1]["text"],
        "inferred by e2 e8; the scope receives True as witness"
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
    let value = json(
        r#"{"verb": "select", "program": {"source": "Seed.A, [Seed] ().([A] B)"}, "pattern": "().([A] B)"}"#,
    );
    assert_eq!(value["answer"]["kind"], "configuration");
    assert!(value["answer"]["total"].as_u64().unwrap_or(0) > 0);
    let scope = json(
        r#"{"verb": "select", "program": {"source": "Seed.A, [Seed] ().([A] B)"}, "pattern": "([A] B)"}"#,
    );
    assert_eq!(scope["answer"]["kind"], "configuration");
    assert_eq!(scope["answer"]["total"], 0);
    let broken =
        json(r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "A B)"}"#);
    assert_eq!(broken["error"]["code"], "pattern");
}

#[test]
fn inspect() {
    for (handle, kind) in [
        ("r2", "rule"),
        ("s6", "configuration"),
        ("s5.c0", "coherence"),
        ("s6.o1", "occurrence"),
        ("s5.f1", "frame"),
        ("e11", "event"),
    ] {
        let value = json(&format!(
            r#"{{"verb": "inspect", "program": {{"file": ["bug.wave"]}}, "handle": "{handle}"}}"#
        ));
        assert_eq!(value["answer"]["kind"], kind, "{handle}");
    }
    let event = json(r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "e4"}"#);
    let witness = event["answer"]["witness"]
        .as_array()
        .expect("witness occurrences")
        .iter()
        .map(|item| item["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(witness, vec!["False", "True"]);
    let produce =
        json(r#"{"verb": "inspect", "program": {"file": ["original.wave"]}, "handle": "e8"}"#);
    assert_eq!(produce["answer"]["produced"][0]["handle"], "s6.o0");
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
    assert_eq!(preserved["handle"], "s6", "{preserved}");
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
    assert_eq!(
        answer[2]["answer"]["claim"][0]["answer"], "holds",
        "{}",
        answer[2]
    );
    assert_eq!(
        answer[2]["answer"]["claim"][0]["witness"], "s0",
        "{}",
        answer[2]
    );
    let near = &answer[3]["answer"]["near"][0];
    assert_eq!(near["handle"], "s0", "{}", answer[3]);
    assert_eq!(near["distance"], 1, "{}", answer[3]);
    assert_eq!(near["extra"], serde_json::json!(["Z"]), "{}", answer[3]);
    assert_eq!(answer[4]["answer"]["total"], 1, "{}", answer[4]);
    assert_eq!(
        answer[4]["answer"]["found"][0]["frame"],
        serde_json::json!(["s0.f1"]),
        "{}",
        answer[4]
    );
    let explored = session(&[
        r#"{"verb": "explore", "program": {"source": "Z, (X, [X] Y), A, [A] (B, [B] C)"}}"#,
        r#"{"verb": "inspect", "program": {"source": "Z, (X, [X] Y), A, [A] (B, [B] C)"}, "handle": "s0.f1"}"#,
    ]);
    let scope = |text: &str| {
        explored[0]["answer"]["rule"]
            .as_array()
            .expect("rules")
            .iter()
            .find(|rule| rule["text"] == text)
            .map(|rule| rule["scope"].clone())
            .expect("the rule is listed")
    };
    let opener = explored[0]["answer"]["rule"]
        .as_array()
        .expect("rules")
        .iter()
        .find(|rule| rule["text"] == "[A] (B, [B] C)")
        .map(|rule| rule["handle"].clone())
        .expect("the opening rule is listed");
    assert_eq!(scope("[X] Y"), "program");
    assert_eq!(scope("[B] C"), opener);
    assert_eq!(scope("[A] (B, [B] C)"), serde_json::Value::Null);
    assert_eq!(
        explored[1]["answer"]["scope"]["opener"], "program",
        "{}",
        explored[1]
    );
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
    assert_eq!(answer["closed"], true);
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
        &format!(r#"{{"verb": "cause", "exploration": "{key}", "handle": "s3"}}"#),
        &mut context,
    );
    assert!(first.contains(&key));
    assert!(again.contains("\"path\""), "{again}");
}

// On metal, explore lists the ends of every plain schedule by their text alone, without handles or
// rule activity, counts the matches its grounding read as its work, and the questions that follow
// events refuse it.
#[test]
fn metal() {
    let value = json(
        r#"{"verb": "explore", "program": {"file": ["light.wave"]}, "mode": "plain", "engine": "metal"}"#,
    );
    let answer = &value["answer"];
    assert_eq!(answer["engine"], "metal");
    assert_eq!(answer["configuration"], 4);
    assert_eq!(answer["event"], 3);
    assert_eq!(answer["endless"], false);
    let end = answer["end"].as_array().expect("end configurations");
    assert_eq!(end.len(), 3);
    assert!(end.iter().all(|end| end.get("handle").is_none()));
    assert_eq!(answer["rule"].as_array().map(Vec::len), Some(0));
    assert!(answer.get("depth").is_none());
    assert_eq!(answer["work"], 3);
    let refused = json(
        r#"{"verb": "select", "program": {"file": ["light.wave"]}, "mode": "plain", "engine": "metal", "pattern": "Red"}"#,
    );
    assert_eq!(refused["error"]["code"], "engine");
    let exhaustive =
        json(r#"{"verb": "explore", "program": {"file": ["light.wave"]}, "engine": "metal"}"#);
    assert_eq!(exhaustive["error"]["code"], "request");
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
    assert_eq!(same.class[0].renaming, vec!["And → Or, (True False)"]);
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
        r#"{"verb": "check", "program": {"source": "A B)"}}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}}"#,
        r#"{"verb": "explore", "program": {"file": ["bug.wave"]}, "mode": "path"}"#,
        r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "False"}"#,
        r#"{"verb": "select", "program": {"file": ["bug.wave"]}, "pattern": "[False] False"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "r2"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s5"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s5.c0"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s6.o1"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "s5.f1"}"#,
        r#"{"verb": "inspect", "program": {"file": ["bug.wave"]}, "handle": "e4"}"#,
        r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s6"}"#,
        r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "e4"}"#,
        r#"{"verb": "cause", "program": {"file": ["bug.wave"]}, "handle": "s6.o1"}"#,
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
        &format!(r#"{{"verb": "cause", "exploration": "{key}", "handle": "s6"}}"#),
        &format!(r#"{{"verb": "cause", "exploration": "{key}", "mode": "path", "handle": "s6"}}"#),
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

// These two programs' keys collide under the hash, and each is still answered from its own
// recording.
#[test]
fn twin() {
    let answer = session(&[
        r#"{"verb": "explore", "program": {"source": "ZebraZebraZebraZ, [ZebraZebraZebraZ] Done"}}"#,
        r#"{"verb": "check", "program": {"source": "Z5tMuu2wrQUnJPL7, [Z5tMuu2wrQUnJPL7] Done"}, "claim": [{"kind": "reach", "pattern": "Z5tMuu2wrQUnJPL7"}]}"#,
    ]);
    assert_eq!(
        answer[0]["answer"]["exploration"], answer[1]["answer"]["summary"]["exploration"],
        "the programs share a key"
    );
    assert_eq!(
        answer[1]["answer"]["claim"][0]["answer"], "holds",
        "{}",
        answer[1]
    );
}

// Parts listed anywhere in a pattern take different coherences and frames, however deep they sit:
// no coherence or scope serves two parts, and the order parts are written in never matters.
#[test]
fn distinct() {
    let answer = session(&[
        r#"{"verb": "check", "program": {"source": "Start, (X, [X] Y)"}, "claim": [{"kind": "reach", "pattern": "X, (X, [X] Y)"}]}"#,
        r#"{"verb": "select", "program": {"source": "Start, (X, [X] Y)"}, "pattern": "(X, [X] Y), X"}"#,
        r#"{"verb": "select", "program": {"source": "Start, (A, [A] Z, (B, [B] W))"}, "pattern": "(A, [A] Z, (B, [B] W)), (B, [B] W)"}"#,
        r#"{"verb": "select", "program": {"source": "Start, X, (X, [X] Y)"}, "pattern": "(X, [X] Y), X"}"#,
        r#"{"verb": "select", "program": {"source": "Start, (A, [A] Z, (B, [B] W), (B, [B] W))"}, "pattern": "(B, [B] W), (A, [A] Z, (B, [B] W))"}"#,
        r#"{"verb": "select", "program": {"source": "Start, (X, X.Z, [X] Y)"}, "pattern": "X.Z, (X, [X] Y)"}"#,
    ]);
    assert_eq!(
        answer[0]["answer"]["claim"][0]["answer"], "fails",
        "{}",
        answer[0]
    );
    for index in [1, 2] {
        assert_eq!(answer[index]["answer"]["total"], 0, "{}", answer[index]);
    }
    for index in [3, 4, 5] {
        assert_eq!(
            answer[index]["answer"]["found"][0]["handle"], "s0",
            "{}",
            answer[index]
        );
    }
    let frame = answer[4]["answer"]["found"][0]["frame"]
        .as_array()
        .expect("the scopes' frames")
        .iter()
        .map(|frame| frame.as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(frame.len(), 3, "{}", answer[4]);
    assert!(
        (1..frame.len()).all(|index| !frame[..index].contains(&frame[index])),
        "{}",
        answer[4]
    );
}

// Configurations compare by which scope holds each coherence, how scopes nest and which rules live
// in each scope, so programs that differ there never compare as identical, and a program compared
// with itself counts every configuration its exploration holds.
#[test]
fn layout() {
    let compare = |left: &str, right: &str| {
        let Ok(Answer::Compare(answer)) = answer(&Request::Compare(crate::compare::Request {
            left: source(left),
            right: source(right),
            claim: Vec::new(),
            limit: 12,
        })) else {
            panic!("compare answers");
        };
        answer
    };
    for (left, right) in [
        ("(X, Y, [Q] R), (Z, [Q] R)", "(X, [Q] R), (Y, Z, [Q] R)"),
        ("(X, [Q] R, (Y, [Q] R))", "(X, [Q] R), (Y, [Q] R)"),
        ("Go, [Go] (X, [Q] R)", "Go, [Go] (X, [X.Q] R)"),
    ] {
        let different = compare(left, right);
        assert!(!different.configuration.same(), "{left} {right}");
        assert!(!different.passed(), "{left} {right}");
        let same = compare(left, left);
        assert!(same.configuration.same() && same.passed(), "{left}");
    }
    let two = "Go, [Go] (X, [Q] R), [Go] (X, [Q] S)";
    let itself = compare(two, two);
    assert_eq!(
        (itself.left.configuration, itself.left.event),
        (3, 2),
        "{}",
        itself.text()
    );
}

// Claims, and questions that cannot be answered as asked, are refused before the program is read or
// explored, and preserve means nothing without an exact target.
#[test]
fn admission() {
    let answer = session(&[
        r#"{"verb": "check", "program": {"file": ["absent.wave"]}, "mode": "plain", "engine": "metal", "claim": [{"kind": "reach", "pattern": "Done"}]}"#,
        r#"{"verb": "check", "program": {"file": ["absent.wave"]}, "mode": "path", "claim": [{"kind": "reach", "pattern": "Done", "exact": true}]}"#,
        r#"{"verb": "check", "program": {"file": ["absent.wave"]}, "claim": [{"kind": "reach", "pattern": "Done", "preserve": true}]}"#,
        r#"{"verb": "compare", "left": {"program": {"file": ["absent.wave"]}, "mode": "plain", "engine": "metal"}, "right": {"program": {"file": ["bug.wave"]}}}"#,
        r#"{"verb": "compare", "left": {"program": {"file": ["bug.wave"]}}, "right": {"program": {"file": ["bug.wave"]}}, "claim": [{"kind": "reach", "pattern": "False.Extra", "preserve": true}]}"#,
        r#"{"verb": "miss", "program": {"file": ["absent.wave"]}, "rule": "r3", "preserve": true}"#,
        r#"{"verb": "miss", "program": {"file": ["absent.wave"]}}"#,
        r#"{"verb": "check", "program": {"file": ["bug.wave"]}, "claim": [{"kind": "reach", "pattern": "False.Extra", "exact": true, "preserve": true}]}"#,
    ]);
    for (index, code) in [
        "claim", "claim", "request", "engine", "request", "request", "request",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(answer[index]["error"]["code"], code, "{}", answer[index]);
    }
    assert!(
        answer[6]["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("name what to explain")),
        "{}",
        answer[6]
    );
    assert_eq!(
        answer[7]["answer"]["claim"][0]["answer"], "holds",
        "{}",
        answer[7]
    );
}

// Nested and sibling scopes read differently, every scope names the frame it sits in, the root is a
// frame with a handle like any other, and rules that open different scopes read differently.
#[test]
fn nesting() {
    let answer = session(&[
        r#"{"verb": "explore", "program": {"source": "Start, (X, [Q] R, (Y, [Q] R))"}}"#,
        r#"{"verb": "explore", "program": {"source": "Start, (X, [Q] R), (Y, [Q] R)"}}"#,
        r#"{"verb": "inspect", "program": {"source": "Start, (X, [Q] R, (Y, [Q] R))"}, "handle": "s0"}"#,
        r#"{"verb": "explore", "program": {"source": "Go, [Go] (X, [Q] R), [Go] (X, [Q] S)"}}"#,
    ]);
    let end = |index: usize| answer[index]["answer"]["end"][0]["text"].clone();
    assert_eq!(end(0), "Start · in f1: X · in f2 in f1: Y", "{}", answer[0]);
    assert_eq!(end(1), "Start · in f1: X · in f2: Y", "{}", answer[1]);
    let inspected = &answer[2]["answer"];
    let frame = inspected["frame"]
        .as_array()
        .expect("the configuration's frames")
        .iter()
        .map(|scope| (scope["handle"].clone(), scope["parent"].clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        frame,
        vec![
            (serde_json::json!("s0.f0"), serde_json::Value::Null),
            (serde_json::json!("s0.f1"), serde_json::json!("s0.f0")),
            (serde_json::json!("s0.f2"), serde_json::json!("s0.f1")),
        ],
        "{inspected}"
    );
    assert!(
        inspected["coherence"]
            .as_array()
            .expect("the configuration's coherences")
            .iter()
            .all(|part| part["frame"]
                .as_str()
                .is_some_and(|frame| frame.starts_with("s0.f"))),
        "{inspected}"
    );
    let opener = answer[3]["answer"]["rule"]
        .as_array()
        .expect("rules")
        .iter()
        .filter_map(|rule| rule["text"].as_str())
        .filter(|text| text.starts_with("[Go]"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(opener.len(), 2, "{}", answer[3]);
}

// An open exploration and a direct path say what they have not settled: neither is closed, a path
// that reaches its goal is reached and still not closed, every answer names its state, and none
// says never of a rule that has not fired.
#[test]
fn extent() {
    let open = r#""program": {"source": "Seed, [Seed] Seed.X, [Done] Stop"}, "budget": {"configuration": 5}"#;
    let path = r#""program": {"file": ["light.wave"]}, "mode": "path", "goal": {"configuration": "Red", "preserve": true}"#;
    let explored = session(&[
        &format!(r#"{{"verb": "explore", {open}}}"#),
        &format!(r#"{{"verb": "explore", {path}}}"#),
    ]);
    let handle = |answer: &serde_json::Value, text: &str| {
        answer["answer"]["rule"]
            .as_array()
            .expect("rules")
            .iter()
            .find(|rule| rule["text"] == text)
            .and_then(|rule| rule["handle"].as_str())
            .expect("the rule is listed")
            .to_owned()
    };
    let stop = handle(&explored[0], "[Done] Stop");
    let green = handle(&explored[1], "[Light] Green");
    let answer = session(&[
        &format!(r#"{{"verb": "explore", {open}}}"#),
        &format!(r#"{{"verb": "inspect", {open}, "handle": "{stop}"}}"#),
        &format!(r#"{{"verb": "select", {open}, "pattern": "Seed"}}"#),
        &format!(r#"{{"verb": "step", {open}}}"#),
        &format!(r#"{{"verb": "explore", {path}}}"#),
        &format!(r#"{{"verb": "inspect", {path}, "handle": "{green}"}}"#),
        &format!(r#"{{"verb": "step", {path}}}"#),
        &format!(r#"{{"verb": "miss", {path}, "target": "Blue"}}"#),
        &format!(r#"{{"verb": "compare", "left": {{{path}}}, "right": {{{path}}}}}"#),
        &format!(r#"{{"verb": "select", {path}, "pattern": "Red"}}"#),
    ]);
    for (index, value) in answer.iter().enumerate() {
        let body = &value["answer"];
        let closed = if body.get("left").is_some() {
            body["left"]["closed"].clone()
        } else {
            body["closed"].clone()
        };
        assert_eq!(closed, false, "{index}: {value}");
    }
    assert_eq!(answer[4]["answer"]["reached"], true, "{}", answer[4]);
    let reader = memory();
    let mut store = Store::default();
    let mut context = Context {
        reader: &reader,
        store: &mut store,
    };
    let text = |request: String, context: &mut Context<'_>| {
        let serde_json::Value::Object(mut object) =
            serde_json::from_str(&request).expect("a JSON object")
        else {
            panic!("a request is an object");
        };
        let verb = object
            .remove("verb")
            .and_then(|verb| verb.as_str().map(str::to_owned))
            .expect("a verb");
        Request::read(&verb, serde_json::Value::Object(object))
            .and_then(|request| request.answer(context))
            .map(|answer| answer.text())
            .expect("the question answers")
    };
    let explore = text(format!(r#"{{"verb": "explore", {open}}}"#), &mut context);
    assert!(explore.contains("not yet"), "{explore}");
    assert!(!explore.contains("never"), "{explore}");
    let inspect = text(
        format!(r#"{{"verb": "inspect", {open}, "handle": "{stop}"}}"#),
        &mut context,
    );
    assert!(inspect.contains("has not fired yet"), "{inspect}");
    let select = text(
        format!(r#"{{"verb": "select", {open}, "pattern": "Seed"}}"#),
        &mut context,
    );
    assert!(
        select
            .lines()
            .next()
            .is_some_and(|line| line.contains(" · open · ")),
        "{select}"
    );
    let step = text(format!(r#"{{"verb": "step", {open}}}"#), &mut context);
    assert!(
        step.contains("the exploration is open; more events may happen here"),
        "{step}"
    );
    let configuration = text(
        format!(r#"{{"verb": "inspect", {open}, "handle": "s0"}}"#),
        &mut context,
    );
    assert!(
        configuration.contains("the exploration is open; more events may happen here"),
        "{configuration}"
    );
    let walked = text(format!(r#"{{"verb": "explore", {path}}}"#), &mut context);
    assert!(walked.contains("not on this path"), "{walked}");
    let taken = text(format!(r#"{{"verb": "step", {path}}}"#), &mut context);
    assert!(
        taken.contains("a direct path records only the event it took"),
        "{taken}"
    );
    let inspect = text(
        format!(r#"{{"verb": "inspect", {path}, "handle": "{green}"}}"#),
        &mut context,
    );
    assert!(inspect.contains("does not fire on this path"), "{inspect}");
    let miss = text(
        format!(r#"{{"verb": "miss", {path}, "target": "Blue"}}"#),
        &mut context,
    );
    assert!(
        miss.lines()
            .next()
            .is_some_and(|line| line.contains(" · a direct path · ") && !line.contains("closed")),
        "{miss}"
    );
}

// In an open exploration a rule that has not fired is not said never to fire: configurations whose
// events were not recorded count as unexplored, and a row where its input matches is marked so.
#[test]
fn unexplored() {
    let open = r#""program": {"source": "N, [N] N.N, [N.N.N] Done"}, "budget": {"work": 10}"#;
    let explored = json(&format!(r#"{{"verb": "explore", {open}}}"#));
    assert_eq!(explored["answer"]["closed"], false, "{explored}");
    let done = explored["answer"]["rule"]
        .as_array()
        .expect("rules")
        .iter()
        .find(|rule| rule["text"] == "[N.N.N] Done")
        .and_then(|rule| rule["handle"].as_str())
        .expect("the rule is listed")
        .to_owned();
    let miss = json(&format!(r#"{{"verb": "miss", {open}, "rule": "{done}"}}"#));
    let answer = &miss["answer"];
    assert_eq!(
        (answer["visible"].clone(), answer["unexplored"].clone()),
        (serde_json::json!(2), serde_json::json!(1)),
        "{miss}"
    );
    let matched = answer["near"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["missing"] == 0)
        .expect("a row where the input matches");
    assert_eq!(matched["unexplored"], true, "{miss}");
}

// A target's distance is 0 exactly where select finds it, however its parts nest, and a target
// with more parts than any configuration holds is refused at once.
#[test]
fn measure() {
    let answer = session(&[
        r#"{"verb": "miss", "program": {"source": "Start, (X, [X] Y)"}, "target": "X, (X, [X] Y)"}"#,
        r#"{"verb": "miss", "program": {"source": "Start, X, (X, [X] Y)"}, "target": "(X, [X] Y), X"}"#,
        &serde_json::json!({"verb": "miss", "program": {"source": "A, [A] B"}, "target": vec!["A"; 1000].join(", ")}).to_string(),
        &serde_json::json!({"verb": "miss", "program": {"source": "A, [A] B"}, "target": vec!["A"; 200].join(", ")}).to_string(),
    ]);
    let near = &answer[0]["answer"]["near"][0];
    assert_ne!(near["distance"], 0, "{}", answer[0]);
    assert_eq!(near["missing"], serde_json::json!(["X"]), "{}", answer[0]);
    assert_eq!(
        answer[1]["answer"]["near"][0]["distance"], 0,
        "{}",
        answer[1]
    );
    assert_eq!(answer[2]["error"]["code"], "pattern", "{}", answer[2]);
    assert_eq!(
        answer[3]["answer"]["near"][0]["distance"], 199,
        "{}",
        answer[3]
    );
}

// Refusals name what to change on both surfaces: the command line's flag and the request's field.
#[test]
fn phrasing() {
    let answer = session(&[
        r#"{"verb": "explore", "program": {"file": ["light.wave"]}, "engine": "metal"}"#,
        r#"{"verb": "explore", "program": {"file": ["light.wave"]}, "goal": {"configuration": "Red"}}"#,
        r#"{"verb": "explore", "program": {"file": ["light.wave"]}, "mode": "path", "engine": "laser"}"#,
        r#"{"verb": "explore", "program": {}}"#,
        r#"{"verb": "cause", "program": {"file": ["light.wave"]}, "mode": "path", "handle": "s0.o0"}"#,
    ]);
    for (index, (flag, field)) in [
        ("--plain", "mode plain"),
        ("--path", "mode path"),
        ("--engine", "engine"),
        ("--source", "source"),
        ("--path", "mode path"),
    ]
    .into_iter()
    .enumerate()
    {
        let message = answer[index]["error"]["message"]
            .as_str()
            .unwrap_or_default();
        assert!(
            message.contains(flag) && message.contains(field),
            "{}",
            answer[index]
        );
    }
}

// A limit of 0 lists nothing, counts everything and offers no next page, and a rule's events
// beyond those inspect lists are counted.
#[test]
fn limit() {
    let grow = r#""program": {"file": ["grow.wave"]}, "budget": {"configuration": 20}"#;
    let answer = session(&[
        r#"{"verb": "select", "program": {"file": ["light.wave"]}, "pattern": "()", "limit": 0}"#,
        r#"{"verb": "explore", "program": {"file": ["light.wave"]}, "limit": 0}"#,
        &format!(r#"{{"verb": "inspect", {grow}, "handle": "r0"}}"#),
    ]);
    let select = &answer[0]["answer"];
    assert_eq!(select["total"], 4, "{select}");
    assert_eq!(select["found"], serde_json::json!([]), "{select}");
    assert!(select.get("next").is_none(), "{select}");
    assert_eq!(answer[1]["answer"]["more"], 3, "{}", answer[1]);
    let inspect = &answer[2]["answer"];
    let listed = inspect["event"].as_array().map_or(0, Vec::len);
    assert_eq!(listed, 8, "{inspect}");
    assert_eq!(inspect["more"], 11, "{inspect}");
    let reader = memory();
    let mut store = Store::default();
    let mut context = Context {
        reader: &reader,
        store: &mut store,
    };
    let text = |request: Request, context: &mut Context<'_>| {
        request
            .answer(context)
            .expect("the question answers")
            .text()
    };
    let explore = text(
        Request::Explore(crate::explore::Request {
            recording: recording("light.wave"),
            limit: 0,
        }),
        &mut context,
    );
    assert!(
        explore.contains("end    3 configurations not listed"),
        "{explore}"
    );
    assert!(!explore.contains("none"), "{explore}");
    let inspect = text(
        Request::Inspect(crate::inspect::Request {
            recording: Recording {
                budget: Some(Budget {
                    configuration: Some(20),
                    ..Budget::default()
                }),
                ..recording("grow.wave")
            },
            handle: "r0".to_owned(),
        }),
        &mut context,
    );
    assert!(inspect.contains("and 11 more events"), "{inspect}");
}

// Shape lists each piece of a renaming apart, prints no blank lines for an empty program, drops
// patterns whose copies are identical as the viewers do, and refuses a node budget of 0.
#[test]
fn outline() {
    let shape = |source: &[&str], node: usize| {
        answer(&Request::Shape(crate::shape::Request {
            program: source
                .iter()
                .map(|source| Subject {
                    source: Some((*source).to_owned()),
                    ..Subject::default()
                })
                .collect(),
            target: None,
            fix: Vec::new(),
            node,
        }))
    };
    let Ok(Answer::Shape(chain)) = shape(&["A, B, [A] X, [B] Y", "C, D, [C] Z, [D] W"], 1_000_000)
    else {
        panic!("shape answers");
    };
    assert_eq!(chain.class[0].renaming.len(), 1);
    assert_eq!(
        chain.class[0].renaming[0].matches(", ").count(),
        3,
        "{}",
        chain.class[0].renaming[0]
    );
    let Ok(Answer::Shape(empty)) = shape(&[""], 1_000_000) else {
        panic!("shape answers");
    };
    assert!(
        !empty.text().contains("\n\n") && !empty.text().ends_with('\n'),
        "{:?}",
        empty.text()
    );
    let Ok(Answer::Shape(twice)) = shape(&["[A] B, [A] B"], 1_000_000) else {
        panic!("shape answers");
    };
    assert!(
        twice
            .form
            .as_ref()
            .is_some_and(|form| form.pattern.is_empty())
    );
    assert!(!twice.text().contains("identical"), "{}", twice.text());
    let Err(refused) = shape(&["[A] B"], 0) else {
        panic!("a node budget of 0 is refused");
    };
    assert_eq!(refused.code, Code::Request);
}

// A program that does not parse is the same object whether check reports it as a diagnostic or
// another question fails with it, and names the frontend's error as the diagnostic.
#[test]
fn diagnostic() {
    let answer = session(&[
        r#"{"verb": "check", "program": {"source": "A B)"}}"#,
        r#"{"verb": "explore", "program": {"source": "A B)"}}"#,
    ]);
    assert_eq!(
        answer[0]["answer"]["diagnostic"][0], answer[1]["error"],
        "{}",
        answer[0]
    );
    assert_eq!(answer[1]["error"]["code"], "source", "{}", answer[1]);
    assert_eq!(answer[1]["error"]["diagnostic"], "syntax", "{}", answer[1]);
}

// Each library and program file loads once, however often it is given, as Bazel loads a library
// that several dependencies share, and a path given both as a library and as a program file is
// refused.
#[test]
fn library() {
    let answer = session(&[
        r#"{"verb": "explore", "program": {"file": ["seed.wave"], "library": ["rule.particle", "./rule.particle"]}}"#,
        r#"{"verb": "explore", "program": {"file": ["seed.wave", "seed.wave"], "library": ["rule.particle"]}}"#,
        r#"{"verb": "explore", "program": {"file": ["rule.particle"], "library": ["rule.particle"]}}"#,
        r#"{"verb": "explore", "program": {"file": ["seed.wave"], "library": ["rule.particle", "rule.particle"]}}"#,
    ]);
    for index in [0, 3] {
        let rule = answer[index]["answer"]["rule"]
            .as_array()
            .map_or(0, Vec::len);
        assert_eq!(rule, 1, "{}", answer[index]);
    }
    assert_eq!(answer[1]["answer"]["end"][0]["text"], "B", "{}", answer[1]);
    assert_eq!(answer[2]["error"]["code"], "request", "{}", answer[2]);
}
