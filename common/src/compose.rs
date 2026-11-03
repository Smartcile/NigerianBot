//! Compose project model shared by the API and (eventually) other surfaces.
//!
//! The dashboard edits a small, typed subset of the Compose spec. We store that
//! subset as JSON and render deterministic YAML from it here — never from the
//! browser — so the written files are stable and validated in one place.
//!
//! Secret handling: a secret env var is rendered as `${KEY}` and referenced
//! through an `env_file` next to the project. Values flagged secret are never
//! rendered or required; the real values live in the host-side `.env` file.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Longest allowed project slug.
pub const SLUG_MAX: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectModel {
    #[serde(default)]
    pub services: Vec<ServiceModel>,
    #[serde(default)]
    pub networks: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ServiceModel {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub environment: Vec<EnvVar>,
    #[serde(default)]
    pub networks: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvVar {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default)]
    pub secret: bool,
}

/// Rendered Compose document (the subset we support).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ComposeFile {
    #[serde(default)]
    pub services: BTreeMap<String, Service>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub networks: BTreeMap<String, Empty>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub volumes: BTreeMap<String, Empty>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Service {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_file: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// A Compose network or volume declaration (we only need the empty form).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Empty {}

/// Validate a project slug (also used as the published filename).
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        bail!("project name is required");
    }
    if slug.len() > SLUG_MAX {
        bail!("project name must be at most {SLUG_MAX} characters");
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        bail!("project name may only contain lowercase letters, digits, '-' and '_'");
    }
    if !slug
        .chars()
        .next()
        .map(|c| c.is_ascii_alphanumeric())
        .unwrap_or(false)
    {
        bail!("project name must start with a letter or digit");
    }
    Ok(())
}

/// Validate the whole project before saving/publishing.
pub fn validate_model(model: &ProjectModel) -> Result<()> {
    if model.services.is_empty() {
        bail!("add at least one service");
    }

    // First pass: collect names so `depends_on` may reference services defined
    // later in the list (Compose does not require declaration order).
    let mut names = BTreeSet::new();
    for service in &model.services {
        if !valid_identifier(&service.name) {
            bail!("invalid service name '{}'", service.name);
        }
        if !names.insert(service.name.as_str()) {
            bail!("duplicate service name '{}'", service.name);
        }
    }

    for service in &model.services {
        let image = service.image.as_deref().unwrap_or("").trim();
        let build = service.build.as_deref().unwrap_or("").trim();
        if image.is_empty() && build.is_empty() {
            bail!("service '{}' needs an image or a build path", service.name);
        }
        for (label, value) in [("image", image), ("build", build)] {
            if !value.is_empty() && value.contains('\n') {
                bail!("service '{}' has an invalid {label}", service.name);
            }
        }
        if let Some(restart) = &service.restart {
            if !matches!(
                restart.as_str(),
                "no" | "always" | "on-failure" | "unless-stopped"
            ) {
                bail!("service '{}' has an invalid restart policy", service.name);
            }
        }
        if let Some(command) = &service.command {
            if command.trim().is_empty() {
                bail!("service '{}' has an empty command", service.name);
            }
        }

        let mut env_keys = BTreeSet::new();
        for env in &service.environment {
            if !valid_env_key(&env.key) {
                bail!(
                    "service '{}' has an invalid environment key '{}'",
                    service.name,
                    env.key
                );
            }
            if !env_keys.insert(env.key.as_str()) {
                bail!(
                    "service '{}' repeats environment key '{}'",
                    service.name,
                    env.key
                );
            }
            if env.value.as_deref().is_some_and(|v| v.contains('\n')) {
                bail!(
                    "service '{}' has a multi-line value for '{}'",
                    service.name,
                    env.key
                );
            }
        }

        for (label, entries) in [
            ("port", &service.ports),
            ("volume", &service.volumes),
            ("network", &service.networks),
        ] {
            for entry in entries {
                if entry.trim().is_empty() || entry.contains('\n') {
                    bail!("service '{}' has an invalid {label}", service.name);
                }
            }
        }
        for network in &service.networks {
            if !valid_identifier(network) {
                bail!(
                    "service '{}' has an invalid network name '{}'",
                    service.name,
                    network
                );
            }
        }
        for volume in &service.volumes {
            if let Some(source) = named_volume(volume) {
                if !valid_identifier(source) {
                    bail!(
                        "service '{}' has an invalid volume name '{}'",
                        service.name,
                        source
                    );
                }
            }
        }

        for dep in &service.depends_on {
            if dep == &service.name {
                bail!("service '{}' cannot depend on itself", service.name);
            }
            if !names.contains(dep.as_str()) {
                bail!(
                    "service '{}' depends on unknown service '{}'",
                    service.name,
                    dep
                );
            }
        }
    }

    for (label, entries) in [("network", &model.networks), ("volume", &model.volumes)] {
        let mut seen = BTreeSet::new();
        for entry in entries {
            if !valid_identifier(entry) {
                bail!("invalid {label} name '{entry}'");
            }
            if !seen.insert(entry.as_str()) {
                bail!("duplicate {label} name '{entry}'");
            }
        }
    }

    Ok(())
}

/// Clear secret values (they belong in the host-side `.env`, not the database).
pub fn sanitize(model: &mut ProjectModel) {
    for service in &mut model.services {
        for env in &mut service.environment {
            if env.secret {
                env.value = None;
            }
        }
    }
}

/// Render the project to Compose YAML. The project name becomes the filename and
/// the `env_file` reference, so it is validated as a slug.
pub fn render(slug: &str, model: &ProjectModel) -> Result<String> {
    validate_slug(slug)?;
    validate_model(model)?;
    let file = to_compose(slug, model);
    serde_yaml_ng::to_string(&file).context("failed to render compose YAML")
}

/// Build the typed Compose document (without serializing it).
pub fn to_compose(slug: &str, model: &ProjectModel) -> ComposeFile {
    let mut services = BTreeMap::new();
    let mut networks: BTreeMap<String, Empty> = BTreeMap::new();
    let mut volumes: BTreeMap<String, Empty> = BTreeMap::new();

    for name in &model.networks {
        networks.insert(name.clone(), Empty {});
    }
    for name in &model.volumes {
        volumes.insert(name.clone(), Empty {});
    }

    for service in &model.services {
        let mut environment = BTreeMap::new();
        let mut has_secret = false;
        for env in &service.environment {
            if env.secret {
                has_secret = true;
                environment.insert(env.key.clone(), format!("${{{}}}", env.key));
            } else {
                environment.insert(env.key.clone(), env.value.clone().unwrap_or_default());
            }
        }

        let mut env_file = Vec::new();
        if has_secret {
            env_file.push(format!("./{slug}.env"));
        }

        for network in &service.networks {
            networks.insert(network.clone(), Empty {});
        }
        for volume in &service.volumes {
            if let Some(name) = named_volume(volume) {
                volumes.insert(name.to_string(), Empty {});
            }
        }

        services.insert(
            service.name.clone(),
            Service {
                image: non_empty(&service.image),
                build: non_empty(&service.build),
                ports: service.ports.clone(),
                volumes: service.volumes.clone(),
                environment,
                env_file,
                networks: service.networks.clone(),
                depends_on: service.depends_on.clone(),
                restart: non_empty(&service.restart),
                command: non_empty(&service.command),
            },
        );
    }

    ComposeFile {
        services,
        networks,
        volumes,
    }
}

/// Keys that need a value in the generated `.env` scaffold.
pub fn secret_keys(model: &ProjectModel) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for service in &model.services {
        for env in &service.environment {
            if env.secret {
                keys.insert(env.key.clone());
            }
        }
    }
    keys.into_iter().collect()
}

fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A short-syntax volume entry whose source is a named volume (not a bind mount).
fn named_volume(entry: &str) -> Option<&str> {
    if !entry.contains(':') {
        return None;
    }
    let source = entry.split(':').next()?.trim();
    if source.is_empty()
        || source.starts_with('/')
        || source.starts_with('.')
        || source.starts_with('~')
        || source.contains('$')
    {
        return None;
    }
    Some(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProjectModel {
        ProjectModel {
            services: vec![ServiceModel {
                name: "web".into(),
                image: Some("nginx:latest".into()),
                ports: vec!["8080:80".into()],
                volumes: vec![
                    "/srv/html:/usr/share/nginx/html:ro".into(),
                    "data:/data".into(),
                ],
                environment: vec![
                    EnvVar {
                        key: "TZ".into(),
                        value: Some("UTC".into()),
                        secret: false,
                    },
                    EnvVar {
                        key: "API_KEY".into(),
                        value: Some("must-not-leak".into()),
                        secret: true,
                    },
                ],
                networks: vec!["proxy".into()],
                restart: Some("unless-stopped".into()),
                ..Default::default()
            }],
            networks: vec!["proxy".into()],
            volumes: vec![],
        }
    }

    #[test]
    fn renders_secrets_via_env_file() {
        let yaml = render("my-app", &sample()).unwrap();
        assert!(yaml.contains("API_KEY: ${API_KEY}"));
        assert!(yaml.contains("env_file:"));
        assert!(yaml.contains("- ./my-app.env"));
        assert!(yaml.contains("TZ: UTC"));
    }

    #[test]
    fn secret_value_never_rendered() {
        let yaml = render("my-app", &sample()).unwrap();
        assert!(!yaml.contains("must-not-leak"));
    }

    #[test]
    fn adds_referenced_networks_and_named_volumes() {
        let yaml = render("my-app", &sample()).unwrap();
        assert!(yaml.contains("proxy: {}"));
        assert!(yaml.contains("data: {}"));
    }

    #[test]
    fn render_is_deterministic() {
        let a = render("my-app", &sample()).unwrap();
        let b = render("my-app", &sample()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rendered_yaml_parses() {
        let yaml = render("my-app", &sample()).unwrap();
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml).unwrap();
        assert!(parsed.get("services").is_some());
    }

    #[test]
    fn rejects_bad_slug() {
        assert!(validate_slug("../escape").is_err());
        assert!(validate_slug("UPPER").is_err());
        assert!(validate_slug("").is_err());
        assert!(validate_slug("ok-name_1").is_ok());
    }

    #[test]
    fn rejects_invalid_models() {
        let mut model = sample();
        model.services[0].image = None;
        model.services[0].build = None;
        assert!(validate_model(&model).is_err());

        let mut model = sample();
        model.services.push(model.services[0].clone());
        assert!(validate_model(&model).is_err());

        let mut model = sample();
        model.services[0].environment.push(EnvVar {
            key: "API_KEY".into(),
            value: None,
            secret: true,
        });
        assert!(validate_model(&model).is_err());
    }

    #[test]
    fn allows_forward_dependencies() {
        let mut model = sample();
        model.services[0].depends_on = vec!["db".into()];
        let mut db = model.services[0].clone();
        db.name = "db".into();
        db.depends_on = vec![];
        model.services.push(db);
        assert!(validate_model(&model).is_ok());
    }

    #[test]
    fn does_not_declare_anonymous_volumes() {
        let mut model = sample();
        model.services[0].volumes = vec!["/data".into()];
        let yaml = render("my-app", &model).unwrap();
        assert!(!yaml.contains("data: {}"));
    }

    #[test]
    fn sanitize_clears_secret_values() {
        let mut model = sample();
        sanitize(&mut model);
        assert_eq!(model.services[0].environment[1].value, None);
        assert_eq!(secret_keys(&model), vec!["API_KEY".to_string()]);
    }
}
