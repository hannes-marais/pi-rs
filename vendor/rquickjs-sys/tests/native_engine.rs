extern crate std;

use self::std::ffi::CString;
use crate as rquickjs_sys;

#[test]
fn evaluates_javascript_and_reports_exceptions() {
    unsafe {
        let runtime = rquickjs_sys::JS_NewRuntime();
        assert!(!runtime.is_null());
        let context = rquickjs_sys::JS_NewContext(runtime);
        assert!(!context.is_null());
        let name = CString::new("native_engine.js").unwrap();
        let eval = |source: &str| {
            let source = CString::new(source).unwrap();
            rquickjs_sys::JS_Eval(
                context,
                source.as_ptr(),
                source.as_bytes().len() as _,
                name.as_ptr(),
                rquickjs_sys::JS_EVAL_TYPE_GLOBAL as i32,
            )
        };
        let value = eval("40 + 2");
        assert!(!rquickjs_sys::JS_IsException(value));
        assert_eq!(rquickjs_sys::JS_VALUE_GET_INT(value), 42);
        rquickjs_sys::JS_FreeValue(context, value);
        let error = eval("throw new Error('expected failure')");
        assert!(rquickjs_sys::JS_IsException(error));
        rquickjs_sys::JS_FreeValue(context, error);
        rquickjs_sys::JS_FreeContext(context);
        rquickjs_sys::JS_FreeRuntime(runtime);
    }
}
