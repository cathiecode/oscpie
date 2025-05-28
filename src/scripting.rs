use crate::prelude::*;
struct Script<'s>(v8::Local<'s, v8::Script>);

struct ScriptContext<'s, 'i> {
    context: v8::Local<'s, v8::Context>,
    context_scope: v8::ContextScope<'i, v8::HandleScope<'s>>,
}

impl<'s, 'i> ScriptContext<'s, 'i>
where
    's: 'i,
{
    pub fn new(isolate_scope: &'i mut v8::HandleScope<'s, ()>) -> Self {
        let context = v8::Context::new(isolate_scope, Default::default());

        let context_scope = v8::ContextScope::new(isolate_scope, context);

        ScriptContext {
            context,
            context_scope,
        }
    }

    fn script(&mut self, script: &str) -> Result<Script<'s>> {
        let Some(code) = v8::String::new(&mut self.context_scope, script) else {
            return Err(anyhow::anyhow!("Failed to create script string"));
        };

        let Some(script) = v8::Script::compile(&mut self.context_scope, code, None) else {
            return Err(anyhow::anyhow!("Failed to compile script"));
        };

        Ok(Script(script))
    }

    fn eval<'c>(&'c mut self, script: &Script<'s>) -> Result<ScriptValue<'s, 'i, 'c>>
    where
        's: 'c,
        'i: 'c,
    {
        let Some(result_or_except) = script.0.run(&mut self.context_scope) else {
            return Err(anyhow::anyhow!("Failed to run script"));
        };

        Ok(ScriptValue {
            value: result_or_except,
            context: self,
        })
    }
}

pub struct ScriptValue<'s, 'i, 'c>
where
    's: 'c,
    'i: 'c,
{
    value: v8::Local<'s, v8::Value>,
    context: &'c mut ScriptContext<'s, 'i>,
}

impl TryInto<String> for ScriptValue<'_, '_, '_> {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<String> {
        self.value
            .to_string(&mut self.context.context_scope)
            .map(|s| s.to_rust_string_lossy(&mut self.context.context_scope))
            .ok_or(anyhow::anyhow!("Value is not a string"))
    }
}

impl TryInto<f64> for ScriptValue<'_, '_, '_> {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<f64> {
        self.value
            .to_number(&mut self.context.context_scope)
            .map(|n| n.value())
            .ok_or(anyhow::anyhow!("Value is not a number"))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;
    use v8::new_default_platform;

    static INIT: Once = Once::new();

    fn init_v8() {
        INIT.call_once(|| {
            let platform = new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
    }

    #[test]
    fn test_script_eval() {
        init_v8();

        let isolate = &mut v8::Isolate::new(Default::default());
        let mut isolate_scope = v8::HandleScope::new(isolate);

        let mut script_context = ScriptContext::new(&mut isolate_scope);

        let script = script_context.script("1 + 2").unwrap();

        let a: f64 = script_context.eval(&script).unwrap().try_into().unwrap();

        assert!((a - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_script_compile_fail() {
        init_v8();

        let isolate = &mut v8::Isolate::new(Default::default());
        let mut isolate_scope = v8::HandleScope::new(isolate);

        let mut script_context = ScriptContext::new(&mut isolate_scope);

        assert!(script_context.script("1 +").is_err());
    }

    #[test]
    fn test_script_eval_string() {
        init_v8();

        let isolate = &mut v8::Isolate::new(Default::default());
        let mut isolate_scope = v8::HandleScope::new(isolate);

        let mut script_context = ScriptContext::new(&mut isolate_scope);

        let script = script_context.script("'Hello, World!'").unwrap();

        let result: String = script_context.eval(&script).unwrap().try_into().unwrap();

        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_script_throw() {
        init_v8();

        let isolate = &mut v8::Isolate::new(Default::default());
        let mut isolate_scope = v8::HandleScope::new(isolate);

        let mut script_context = ScriptContext::new(&mut isolate_scope);

        let script = script_context
            .script("throw new Error('Test error')")
            .unwrap();

        assert!(script_context.eval(&script).is_err());
    }

    #[test]
    fn test_script_persistent() {
        init_v8();

        let isolate = &mut v8::Isolate::new(Default::default());
        let mut isolate_scope = v8::HandleScope::new(isolate);

        let mut script_context = ScriptContext::new(&mut isolate_scope);

        let script1 = script_context.script("let x = 16;").unwrap();

        let script2 = script_context.script("x").unwrap();

        script_context.eval(&script1).unwrap();

        let result: f64 = script_context.eval(&script2).unwrap().try_into().unwrap();

        assert_eq!(result, 16.0);
    }
}
