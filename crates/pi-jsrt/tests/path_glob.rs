//! Fixed Node v22.15.1 reference cases; no Node runtime is required.
use pi_jsrt::{JsRuntime, RuntimeOptions};
use serde_json::Value;

#[tokio::test]
async fn path_glob_reference_corpus() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/path-glob-node-22.15.1.json")).unwrap();
    let literal = serde_json::to_string(&cases).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let runtime = JsRuntime::new(RuntimeOptions {
        cwd: Some(temp.path().canonicalize().unwrap()),
        ..Default::default()
    })
    .unwrap();
    let source = format!(
        r#"
        import path from 'node:path';
        const cases = {literal};
        export default cases.map((c) => {{
            try {{ return {{value: path[c.mode].matchesGlob(c.path, c.pattern)}}; }}
            catch (e) {{ return {{error: e.code || e.name}}; }}
        }});
    "#
    );
    let module = runtime
        .eval_module(&source, "path-glob-reference.mjs")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let result: Value = module.get("default").await.unwrap();
    let actual = result.as_array().expect("results");
    assert_eq!(actual.len(), cases.len());
    let expected = if cfg!(any(target_os = "windows", target_os = "macos")) {
        "other"
    } else {
        "linux"
    };
    for (index, (case, observed)) in cases.iter().zip(actual).enumerate() {
        assert_eq!(&case[expected], observed, "case {index}: {case}");
    }
}
