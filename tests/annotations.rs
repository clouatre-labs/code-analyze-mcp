use code_analyze_mcp::CodeAnalyzer;

#[test]
fn test_all_tools_have_correct_annotations() {
    let tools = CodeAnalyzer::list_tools();

    assert_eq!(tools.len(), 4, "expected 4 registered tools");

    let expected_names = [
        "analyze_directory",
        "analyze_file",
        "analyze_module",
        "analyze_symbol",
    ];

    for tool in &tools {
        let name = tool.name.as_ref();
        assert!(
            expected_names.contains(&name),
            "unexpected tool name: {}",
            name
        );

        let annotations = tool
            .annotations
            .as_ref()
            .unwrap_or_else(|| panic!("tool {} is missing annotations", name));

        assert_eq!(
            annotations.read_only_hint,
            Some(true),
            "tool {} must have read_only_hint=true",
            name
        );
        assert_eq!(
            annotations.destructive_hint,
            Some(false),
            "tool {} must have destructive_hint=false",
            name
        );
        assert_eq!(
            annotations.idempotent_hint,
            Some(true),
            "tool {} must have idempotent_hint=true",
            name
        );
        assert_eq!(
            annotations.open_world_hint,
            Some(false),
            "tool {} must have open_world_hint=false",
            name
        );
    }
}
