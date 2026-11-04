//! Fixed Node v22.15.1 reference cases. Node is not needed to run this test.
use pi_jsrt::{JsRuntime, RuntimeOptions};
use serde_json::{Value, json};

#[tokio::test]
async fn windows_path_reference_corpus() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/win32-path-node-22.15.1.json")).unwrap();
    let literal = serde_json::to_string(&cases).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let runtime = JsRuntime::new(RuntimeOptions {
        cwd: Some(temp.path().canonicalize().unwrap()),
        ..Default::default()
    })
    .unwrap();
    let source = format!(
        r#"
        import native from 'node:path';
        import win from 'node:path/win32';
        import posix from 'node:path/posix';
        const cases = {literal};
        const actual = cases.map((c) => {{
            try {{ return {{value: win[c.method](...c.args)}}; }}
            catch (e) {{ return {{error: e.code || e.name}}; }}
        }});
        export default {{ actual, nativeSep: native.sep,
            winSep: win.sep, posixSep: posix.sep,
            winIdentity: native.win32 === win, posixIdentity: native.posix === posix }};
    "#
    );
    let module = runtime
        .eval_module(&source, "win32-reference.mjs")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let result: Value = module.get("default").await.unwrap();
    let actual = result["actual"].as_array().expect("results");
    assert_eq!(actual.len(), cases.len());
    for (index, (case, observed)) in cases.iter().zip(actual).enumerate() {
        assert_eq!(&case["expected"], observed, "case {index}: {case}");
    }
    assert_eq!(result["winSep"], json!("\\"));
    assert_eq!(result["posixSep"], json!("/"));
    assert_eq!(
        result["nativeSep"],
        json!(if cfg!(windows) { "\\" } else { "/" })
    );
    assert_eq!(result["winIdentity"], json!(true));
    assert_eq!(result["posixIdentity"], json!(true));
}
