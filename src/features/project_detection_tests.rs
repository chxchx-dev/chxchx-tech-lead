use super::*;

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("chxchx-detect-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (path, contents) in files {
        let file = root.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, contents).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    root
}

fn stacks(root: &Path) -> Vec<String> {
    detect_project(root).stacks
}

#[test]
fn a_project_that_trains_or_runs_models_is_an_ml_project_not_only_an_ai_one() {
    for (name, requirements) in [
        ("torch", "torch==2.5.0\nnumpy\n"),
        ("hf", "transformers>=4.40\ndatasets\n"),
        ("peft", "peft\ntrl\n"),
        ("serving", "vllm==0.6.0\n"),
        ("track", "wandb\n"),
    ] {
        let root = project(name, &[("requirements.txt", requirements)]);
        assert!(
            stacks(&root).contains(&"ml".to_owned()),
            "{name}: {:?}",
            stacks(&root)
        );
    }
    let toml = project(
        "toml",
        &[(
            "pyproject.toml",
            "[project]\ndependencies = [\"jax\", \"flax\"]\n",
        )],
    );
    assert!(stacks(&toml).contains(&"ml".to_owned()));
}

#[test]
fn a_hosted_api_client_is_ai_but_not_ml() {
    let root = project(
        "api",
        &[("requirements.txt", "openai\nanthropic\nrequests\n")],
    );
    let found = stacks(&root);
    assert!(
        found.contains(&"ai".to_owned()) && !found.contains(&"ml".to_owned()),
        "{found:?}"
    );
    let plain = project("plain", &[("requirements.txt", "requests\nflask\n")]);
    assert!(!stacks(&plain).iter().any(|s| s == "ml" || s == "ai"));
}

#[test]
fn kubernetes_is_found_by_its_charts_kustomizations_and_manifest_folders() {
    for (name, files) in [
        ("chart", vec![("Chart.yaml", "name: app\n")]),
        ("kustomize", vec![("kustomization.yaml", "resources: []\n")]),
        ("k8s", vec![("k8s/deployment.yaml", "kind: Deployment\n")]),
        ("helm", vec![("helm/values.yml", "replicas: 1\n")]),
    ] {
        let root = project(name, &files);
        assert!(stacks(&root).contains(&"kubernetes".to_owned()), "{name}");
    }
    // A YAML file elsewhere (CI, compose) is not Kubernetes.
    let ci = project(
        "ci",
        &[
            (".github/workflows/ci.yaml", "name: ci\n"),
            ("docs/a.yaml", "x: 1\n"),
        ],
    );
    assert!(!stacks(&ci).contains(&"kubernetes".to_owned()));
}

#[test]
fn terraform_is_found_at_the_root_or_in_its_usual_folders() {
    for (name, path) in [
        ("root", "main.tf"),
        ("folder", "terraform/main.tf"),
        ("infra", "infra/vpc.tf.json"),
    ] {
        let root = project(name, &[(path, "")]);
        assert!(stacks(&root).contains(&"terraform".to_owned()), "{name}");
    }
    let none = project("none", &[("src/main.rs", "fn main() {}\n")]);
    assert!(!stacks(&none).contains(&"terraform".to_owned()));
}
