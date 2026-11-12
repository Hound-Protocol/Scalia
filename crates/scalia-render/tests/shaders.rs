use scalia_render::shaders::{BLIT_WGSL, SCENE_WGSL, SHADOW_WGSL};

fn validate(name: &str, src: &str) {
    let module = naga::front::wgsl::parse_str(src).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(src)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
}

#[test]
fn all_shaders_parse_and_validate() {
    validate("scene", &SCENE_WGSL);
    validate("shadow", SHADOW_WGSL);
    validate("blit", BLIT_WGSL);
}
