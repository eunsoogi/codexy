use crate::support;

// Windows uses native executable names while selected runtime metadata remains the activation gate.
#[test]
fn candidate_keeps_windows_native_until_verified_activation()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = super::workflow("runtime-candidate.yml")?;
    let matrix = candidate["jobs"]["build-runtime"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .ok_or("candidate build matrix")?;
    assert!(matrix.iter().any(|entry| {
        entry["platform"] == "windows-x86_64" && entry["runner"] == "windows-latest"
    }));
    let steps = candidate["jobs"]["build-runtime"]["steps"]
        .as_sequence()
        .ok_or("candidate build steps")?;
    let (_, native) = super::named_step(steps, "Smoke native Windows MCP protocols")?;
    assert_eq!(native["shell"], "pwsh");
    support::assert_structured_literals(
        native["run"].as_str().ok_or("native Windows smoke")?,
        "native Windows candidate proof",
        &[
            "ProcessStartInfo",
            "codexy-mcp-lsp.exe",
            "codexy-mcp-codegraph.exe",
            "tools/call",
        ],
    );
    support::assert_structured_absent_literals(
        native["run"].as_str().ok_or("native Windows smoke")?,
        "native Windows candidate smoke omits the retired Watcher runtime",
        &["codexy-mcp-watcher", "watcher_open", "coreWatcherMcp"],
    );
    let assembly = super::run(
        &candidate,
        "stage-runtime",
        "Assemble canonical staged archive and receipt",
    )?;
    assert_eq!(assembly, "scripts/assemble-runtime-candidate");
    let assembly = super::script("assemble-runtime-candidate")?;
    support::assert_structured_literals(
        &assembly,
        "candidate-only Windows activation staging",
        &[
            "windows-x86_64",
            "extension = \"exe\" if platform == \"windows-x86_64\" else \"bin\"",
            "manifest[\"supportedPlatforms\"] = [\"darwin-arm64\", \"linux-x86_64\", \"windows-x86_64\"]",
            "codexy-mcp-devtools-windows-x86_64.exe",
            "codexy-mcp-devtools.exe",
        ],
    );

    let selected = super::workflow("plugin-runtime-binaries.yml")?;
    let windows = super::run(
        &selected,
        "verify-windows-selected-candidate",
        "Verify immutable native Windows candidate bytes",
    )?;
    support::assert_structured_literals(
        windows,
        "selected Windows runtime truth boundary",
        &[
            "legacy-public baseline intentionally has no selected Windows candidate",
            "candidate-proven",
            "tar.exe",
            "codexy-mcp-$server-windows-x86_64.exe",
        ],
    );
    Ok(())
}
