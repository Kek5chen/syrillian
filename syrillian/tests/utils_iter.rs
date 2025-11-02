use syrillian::utils::iter::{Interpolatable, extend_data, extract_data, interpolate_zeros};

#[test]
fn interpolate_zeros_expands_vectors() {
    let mut a: Vec<f32> = vec![1.0];
    let mut b: Vec<i32> = vec![1, 2];
    let mut c: Vec<u8> = Vec::new();

    let mut refs: [&mut dyn Interpolatable; 3] = [&mut a, &mut b, &mut c];
    interpolate_zeros(4, &mut refs);

    assert_eq!(a.len(), 4);
    assert_eq!(b.len(), 4);
    assert_eq!(c.len(), 4);
    assert_eq!(a[1], 0.0);
    assert_eq!(b[2], 0);
    assert_eq!(c[3], 0);
}

#[test]
fn extract_data_returns_converted_subset() {
    let source = vec!["zero", "one", "two", "three"];
    let indices = [0_u32, 2, 3];

    let result = extract_data(&indices, &source, |s| s.len());
    assert_eq!(result, vec![4, 3, 5]);
}

#[test]
fn extend_data_appends_filtered_entries() {
    let mut target: Vec<i32> = vec![1];
    let indices = [1_u32, 3, 99];
    let source = vec![10, 20, 30, 40];

    extend_data(&mut target, &indices, &source, |v| v + 1);

    assert_eq!(target, vec![1, 21, 41]);
}
