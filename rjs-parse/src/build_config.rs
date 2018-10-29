use client_config::ModuleId;
use parse::ConfigParseError;
use parse::ParsedConfig;
use serde_json;
use std::collections::HashMap;
use Module;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequireJsBuildConfig {
    #[serde(rename = "generateSourceMaps")]
    pub generate_source_maps: Option<bool>,

    #[serde(default = "default_inline_text", rename = "inlineText")]
    pub inline_text: Option<bool>,

    #[serde(default = "default_optimize")]
    pub optimize: Option<String>,

    pub deps: Vec<ModuleId>,
    pub map: serde_json::Value,
    pub config: serde_json::Value,
    pub shim: serde_json::Value,
    pub paths: HashMap<String, String>,

    #[serde(default = "default_modules")]
    pub modules: Option<Vec<Module>>,
}

impl RequireJsBuildConfig {
    pub fn from_generated_string(
        input: impl Into<String>,
    ) -> Result<RequireJsBuildConfig, ConfigParseError> {
        let output = ParsedConfig::from_str(input)?;
        let as_serde = serde_json::to_value(&output).map_err(|_e| ConfigParseError::Serialize)?;
        let mut as_rjs: RequireJsBuildConfig =
            serde_json::from_value(as_serde).map_err(|_e| ConfigParseError::Conversion)?;
        as_rjs.paths = RequireJsBuildConfig::strip_paths(&as_rjs.paths);
        as_rjs.modules = Some(vec![Module {
            name: "requirejs/require".into(),
            include: vec![],
            exclude: vec![],
            create: false,
        }]);
        Ok(as_rjs)
    }
    pub fn to_string(&self) -> Result<String, String> {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => Ok(s),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn strip_paths(paths: &HashMap<String, String>) -> HashMap<String, String> {
        let mut hm: HashMap<String, String> = HashMap::new();

        for (key, value) in paths.iter() {
            if value.starts_with("http://")
                || value.starts_with("https://")
                || value.starts_with("//")
            {
                hm.insert(key.clone(), "empty:".to_string());
            } else {
                hm.insert(key.clone(), value.clone());
            }
        }

        hm
    }
    pub fn bundle_loaders(mixins: Vec<String>, modules: Vec<Module>) -> String {
        let items: Vec<String> = modules
            .iter()
            .filter(|m| m.name.as_str() != "requirejs/require")
            .map(|module| {
                let module_list: Vec<String> = module
                    .include
                    .iter()
                    .map(|name| {
                        let is_mixin_trigger = mixins.contains(&name);
                        match is_mixin_trigger {
                            true => format!("         // mixin trigger: \"{}\",", name),
                            false => format!("        \"{}\",", name),
                        }
                    })
                    .collect();

                format!(
                    "require.config({{\n  bundles: {{\n    \"{}\": [\n{}\n    ]\n  }}\n}});",
                    module.name,
                    module_list.join("\n")
                )
            })
            .collect();
        items.join("\n")
    }
    pub fn mixins(val: &serde_json::Value) -> Vec<String> {
        match *val {
            serde_json::Value::Object(ref v) => match v.get("mixins") {
                Some(f) => match f {
                    serde_json::Value::Object(ref v) => {
                        let names: Vec<String> = v.iter().map(|(key, _)| key.to_string()).collect();
                        names
                    }
                    _ => vec![],
                },
                None => vec![],
            },
            _ => vec![],
        }
    }
}

impl Default for RequireJsBuildConfig {
    fn default() -> RequireJsBuildConfig {
        RequireJsBuildConfig {
            deps: vec![],
            map: json!({}),
            config: json!({}),
            shim: json!({}),
            paths: HashMap::new(),
            generate_source_maps: Some(true),
            inline_text: Some(true),
            optimize: Some("uglify".into()),
            modules: Some(vec![]),
        }
    }
}

fn default_optimize() -> Option<String> {
    Some("uglify".to_string())
}
fn default_inline_text() -> Option<bool> {
    Some(true)
}
fn default_modules() -> Option<Vec<Module>> {
    Some(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_mixins() {
        let input = include_str!("../test/fixtures/requirejs-config-generated.js");
        let s = RequireJsBuildConfig::from_generated_string(input).expect("fixture unwrap");
        assert_eq!(
            RequireJsBuildConfig::mixins(&s.config),
            vec![
                "Magento_Checkout/js/action/place-order",
                "Magento_Checkout/js/action/set-payment-information",
                "jquery/jstree/jquery.jstree",
            ]
        );
    }

    #[test]
    fn test_module_list() {
        let list = RequireJsBuildConfig::bundle_loaders(
            vec!["js/shane".to_string()],
            vec![
                Module {
                    name: String::from("requirejs/require"),
                    include: vec![],
                    exclude: vec![],
                    create: false,
                },
                Module {
                    name: String::from("bundle/base"),
                    include: vec!["js/shane".to_string(), "js/kittie".to_string()],
                    exclude: vec![],
                    create: true,
                },
                Module {
                    name: String::from("bundle/product"),
                    include: vec!["js/gallery".to_string(), "js/zoomer".to_string()],
                    exclude: vec![],
                    create: true,
                },
            ],
        );
        let expected = r#"require.config({
  bundles: {
    "bundle/base": [
         // mixin trigger: "js/shane",
        "js/kittie",
    ]
  }
});
require.config({
  bundles: {
    "bundle/product": [
        "js/gallery",
        "js/zoomer",
    ]
  }
});"#;
        assert_eq!(list, expected);
    }

    #[test]
    fn test_strip_paths() {
        let mut ps: HashMap<String, String> = HashMap::new();
        ps.insert("one".into(), "one/one".into());
        ps.insert("two".into(), "http://two.com/two".into());

        let mut expected: HashMap<String, String> = HashMap::new();
        expected.insert("one".into(), "one/one".into());
        expected.insert("two".into(), "empty:".into());

        let actual = RequireJsBuildConfig::strip_paths(&ps);

        assert_eq!(actual, expected);
    }
}