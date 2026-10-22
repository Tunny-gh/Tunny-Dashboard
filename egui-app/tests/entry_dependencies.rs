//! D7 production boundaries exclude the legacy fixture dependency, not its tests.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
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
    for (name, dependencies) in expected {
        let (&id, package) = packages.iter().find(|(_, p)| p["name"] == name).unwrap();
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
    let gui = packages
        .values()
        .find(|p| p["name"] == "tunny-dashboard")
        .unwrap();
    let legacy: Vec<_> = gui["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["name"] == "tunny-core")
        .collect();
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0]["kind"], "dev");
}
