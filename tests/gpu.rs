use lapctl::commands::gpu::{generate_xrandr_script, xorg_amd, xorg_intel};

#[test]
fn test_xorg_intel_generation() {
    let output = xorg_intel("01:00.0");
    assert!(output.contains(r#"Identifier "intel""#));
    assert!(output.contains(r#"BusID "01:00.0""#));
}

#[test]
fn test_xorg_amd_generation() {
    let output = xorg_amd("02:00.0");
    assert!(output.contains(r#"Identifier "amdgpu""#));
    assert!(output.contains(r#"BusID "02:00.0""#));
}

#[test]
fn test_xrandr_script() {
    let intel_str = "intel".to_string();
    let script_with_intel = generate_xrandr_script(Some(&intel_str));
    assert!(
        script_with_intel.contains(r#"xrandr --setprovideroutputsource "modesetting" NVIDIA-0"#)
    );

    let script_without_igpu = generate_xrandr_script(None);
    assert!(
        script_without_igpu.contains(r#"xrandr --setprovideroutputsource "modesetting" NVIDIA-0"#)
    );
}
