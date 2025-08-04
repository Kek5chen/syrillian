use syrillian::audio::audio:: {
    AudioScene, AudioSystem
};

#[test]
fn test_load_sounds() {
    let mut audio = AudioSystem::new();
    audio.load_sound("pop", "examples/assets/pop.wav");
    assert!(audio.has_asset("pop"));
}

#[test]
fn test_calculate_volume_linear() {
    // Within range
    let max_distance = 50.0;
    let distance = 25.0;
    let volume = AudioScene::calculate_volume_linear(distance, max_distance);

    const EPS: f32 = 1e-5;
    assert!((volume - 0.5).abs() < EPS);

    // Out of range
    let max_distance = 50.0;
    let distance = 100.0;
    let volume = AudioScene::calculate_volume_linear(distance, max_distance);
    assert_eq!(volume, 0.0);
}

#[test]
fn test_volume_to_db() {
    // Out of range
    let vol_low = AudioScene::volume_to_db(-1.0);
    assert_eq!(vol_low, -60.0);

    // Zero
    let vol_zero = AudioScene::volume_to_db(0.0);
    assert_eq!(vol_zero, -60.0);

    // In range
    let vol_mid = AudioScene::volume_to_db(0.5);
    assert!(vol_mid > -60.0 && vol_mid < 0.0);

    // One
    let vol_high = AudioScene::volume_to_db(2.0);
    assert_eq!(vol_high, 0.0);

    // Out of range
    let vol_1 = AudioScene::volume_to_db(1.0);
    assert_eq!(vol_1, 0.0);
}