//! ADR-0021 dependency/executable boundaries and ADR-0025 workspace placement.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

#[test]
fn product_entries_use_the_approved_production_graph_and_binary_names() {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let workspace_root = Path::new(metadata["workspace_root"].as_str().unwrap());
    let crates_dir = workspace_root.join("crates");
    let packages: BTreeMap<_, _> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p))
        .collect();
    let nodes: BTreeMap<_, _> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n))
        .collect();
    let workspace: BTreeSet<_> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    let production_edges = |id| -> Vec<&str> {
        nodes[&id]["deps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| {
                d["dep_kinds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|k| k["kind"].is_null() || k["kind"] == "build")
            })
            .map(|d| d["pkg"].as_str().unwrap())
            .collect()
    };
    let expected: [(&str, &[&str]); 6] = [
        (
            "tunny-dashboard",
            &[
                "tdashboard-study",
                "tdashboard-io",
                "tdashboard-analysis",
                "tdashboard-optimization-runner",
            ],
        ),
        (
            "tdashboard-mcp",
            &["tdashboard-study", "tdashboard-io", "tdashboard-analysis"],
        ),
        (
            "tdashboard-optimization-runner",
            &["tdashboard-study", "tdashboard-io", "tdashboard-analysis"],
        ),
        ("tdashboard-io", &["tdashboard-study"]),
        ("tdashboard-analysis", &["tdashboard-study"]),
        ("tdashboard-study", &[]),
    ];
    assert_eq!(workspace.len(), expected.len());
    assert_eq!(
        workspace
            .iter()
            .map(|id| packages[id]["name"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        expected.iter().map(|(name, _)| *name).collect()
    );
    assert!(!packages.values().any(|p| p["name"] == "tunny-core"));
    let workspace_names: BTreeSet<_> = expected.iter().map(|(name, _)| *name).collect();
    for &root in &workspace {
        let name = packages[root]["name"].as_str().unwrap();
        for dependency in packages[root]["dependencies"].as_array().unwrap() {
            let target = dependency["name"].as_str().unwrap();
            if name == "tdashboard-study" {
                assert!(
                    !workspace_names.contains(target),
                    "Study dependency: {target}"
                );
            }
            assert!(
                !matches!(
                    (name, target),
                    ("tdashboard-io", "tdashboard-analysis")
                        | ("tdashboard-analysis", "tdashboard-io")
                ),
                "I/O and analysis must remain independent, including test dependencies"
            );
        }
        let mut pending = production_edges(root);
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            assert_ne!(id, root, "production dependency cycle for {name}");
            if seen.insert(id) {
                pending.extend(production_edges(id));
            }
        }
    }
    for (name, dependencies) in expected {
        let (&id, package) = packages.iter().find(|(_, p)| p["name"] == name).unwrap();
        let directory = match name {
            "tunny-dashboard" => "desktop",
            "tdashboard-mcp" => "mcp",
            "tdashboard-study" => "study",
            "tdashboard-io" => "io",
            "tdashboard-analysis" => "analysis",
            "tdashboard-optimization-runner" => "optimization-runner",
            _ => unreachable!(),
        };
        assert_eq!(
            Path::new(package["manifest_path"].as_str().unwrap()),
            crates_dir.join(directory).join("Cargo.toml"),
            "manifest placement for {name}"
        );
        if name == "tunny-dashboard" {
            let libraries: Vec<_> = package["targets"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| t["kind"].as_array().unwrap().iter().any(|k| k == "lib"))
                .map(|t| t["name"].as_str().unwrap())
                .collect();
            assert_eq!(libraries, ["tunny_dashboard"]);
        }
        let actual: BTreeSet<_> = production_edges(id)
            .into_iter()
            .filter(|id| workspace.contains(id))
            .map(|id| packages[id]["name"].as_str().unwrap())
            .collect();
        assert_eq!(actual, dependencies.iter().copied().collect(), "{name}");
        if let Some(binary) = match name {
            "tunny-dashboard" => Some("TunnyDashboard"),
            "tdashboard-mcp" => Some("tunny-mcp"),
            _ => None,
        } {
            assert!(package["targets"].as_array().unwrap().iter().any(|t| {
                t["name"] == binary && t["kind"].as_array().unwrap().iter().any(|k| k == "bin")
            }));
            let mut pending = vec![id];
            let mut seen = BTreeSet::new();
            while let Some(id) = pending.pop() {
                if seen.insert(id) {
                    assert_ne!(
                        packages[id]["name"], "tunny-core",
                        "{name} production closure"
                    );
                    pending.extend(production_edges(id));
                }
            }
        }
    }
}
