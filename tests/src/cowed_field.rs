#[allow(clippy::enum_variant_names)]
mod foo {
    include!(concat!(env!("OUT_DIR"), "/cowed_field.rs"));
}

#[test]
/// Confirm `Foo` is cowed by creating an instance
fn test_foo_is_cowed() {
    use crate::cowed_field::foo::*;
    use prost::Message;
    use std::borrow::Cow;
    use std::collections::{BTreeMap, HashMap};

    // Define a static byte array for testing
    static STATIC_ARRAY: [u8; 2] = [99, 98];
    let cow: Cow<'static, [u8]> = Cow::Borrowed(&STATIC_ARRAY);

    // Validate Bar different types
    let _ = Bar {
        my_bar_str: "test".to_string(),
        my_bar_cow_str: Cow::Owned("cow_str".to_string()),
        my_bar_bytes: prost::bytes::Bytes::from("Hello world"),
        my_bar_vec: vec![1, 2, 3],
        my_bar_cow_bytes: Cow::Borrowed(&STATIC_ARRAY),
        my_normal_map: HashMap::from([(5, "normal_map".to_string())]),
        my_normal_cow_map: HashMap::from([(5, Cow::Borrowed("normal_cow_map"))]),
        my_btree_map: BTreeMap::from([(5, "btree_map".to_string())]),
        my_btree_cow_map: BTreeMap::from([(5, Cow::Borrowed("btree_cow_map"))]),
    };

    // Build Foo with a mix of Owned and Borrowed Cow variants
    let f = Foo {
        my_str: Cow::Owned("world".to_string()),
        my_int: 5,
        my_repeat: vec![Cow::Borrowed("hello")],
        my_first_map: HashMap::from([(5, Cow::Borrowed("first_map"))]),
        my_second_map: HashMap::from([(Cow::Borrowed("second_map"), 5)]),
        my_third_map: HashMap::from([(
            Cow::Borrowed("third_map_key"),
            Cow::Borrowed("third_map_value"),
        )]),
        my_opt_str: None,
        before: None,
        my_bytes: Cow::Borrowed(&[1, 2, 3]),
        google_str: Some("google".to_string()),
        my_bytes_map: HashMap::from([(7, cow)]),
        my_vec_bytes: Cow::Borrowed(&[4, 5, 6]),
        extra_details: Some(foo::ExtraDetails::OneOfStr(Cow::Borrowed(
            "ExtraDetailsStr",
        ))),
    };

    // Encode the Foo instance to a byte vector
    let encoded = f.encode_to_vec();

    let g = Foo::decode(encoded.as_ref()).expect("Decoding failed");

    // === Assertions for Equality ===

    // Assert that `my_str` fields are equal
    assert_eq!(
        f.my_str.as_ref(),
        g.my_str.as_ref(),
        "my_str fields do not match"
    );

    // Assert that `my_int` fields are equal
    assert_eq!(f.my_int, g.my_int, "my_int fields do not match");

    // Assert that `my_repeat` vectors are equal
    assert_eq!(
        f.my_repeat.len(),
        g.my_repeat.len(),
        "my_repeat lengths do not match"
    );
    for (i, (item_f, item_g)) in f.my_repeat.iter().zip(g.my_repeat.iter()).enumerate() {
        assert_eq!(
            item_f.as_ref(),
            item_g.as_ref(),
            "my_repeat[{}] elements do not match",
            i
        );
    }

    // Assert that `my_first_map` maps are equal
    assert_eq!(
        f.my_first_map.len(),
        g.my_first_map.len(),
        "my_first_map lengths do not match"
    );
    for (key, val_f) in &f.my_first_map {
        let val_g = g
            .my_first_map
            .get(key)
            .expect("Key missing in g.my_first_map");
        assert_eq!(
            val_f.as_ref(),
            val_g.as_ref(),
            "my_first_map values for key {:?} do not match",
            key
        );
    }

    // Assert that `my_second_map` maps are equal
    assert_eq!(
        f.my_second_map.len(),
        g.my_second_map.len(),
        "my_second_map lengths do not match"
    );
    for (key_f, val_f) in &f.my_second_map {
        let val_g = g
            .my_second_map
            .get(key_f)
            .expect("Key missing in g.my_second_map");
        assert_eq!(
            *val_f, *val_g,
            "my_second_map values for key {:?} do not match",
            key_f
        );
    }

    // Assert that `my_third_map` maps are equal
    assert_eq!(
        f.my_third_map.len(),
        g.my_third_map.len(),
        "my_third_map lengths do not match"
    );
    for (key_f, val_f) in &f.my_third_map {
        let (key_g, val_g) = g
            .my_third_map
            .get_key_value(key_f)
            .expect("Key missing in g.my_third_map");
        assert_eq!(
            key_f.as_ref(),
            key_g.as_ref(),
            "my_third_map keys do not match"
        );
        assert_eq!(
            val_f.as_ref(),
            val_g.as_ref(),
            "my_third_map values do not match"
        );
    }

    // Assert that `my_opt_str` fields are equal
    assert_eq!(f.my_opt_str, g.my_opt_str, "my_opt_str fields do not match");

    // Assert that `before` fields are equal
    assert_eq!(f.before, g.before, "before fields do not match");

    // Assert that `my_bytes` fields are equal
    assert_eq!(
        f.my_bytes.as_ref(),
        g.my_bytes.as_ref(),
        "my_bytes fields do not match"
    );

    // Assert that `google_str` fields are equal
    assert_eq!(f.google_str, g.google_str, "google_str fields do not match");

    // Assert that `my_bytes_map` maps are equal
    assert_eq!(
        f.my_bytes_map.len(),
        g.my_bytes_map.len(),
        "my_bytes_map lengths do not match"
    );
    for (key, val_f) in &f.my_bytes_map {
        let val_g = g
            .my_bytes_map
            .get(key)
            .expect("Key missing in g.my_bytes_map");
        assert_eq!(
            val_f.as_ref(),
            val_g.as_ref(),
            "my_bytes_map values for key {:?} do not match",
            key
        );
    }

    // Assert that `my_vec_bytes` fields are equal
    assert_eq!(
        f.my_vec_bytes.as_ref(),
        g.my_vec_bytes.as_ref(),
        "my_vec_bytes fields do not match"
    );

    // Assert that `extra_details` fields are equal
    match (&f.extra_details, &g.extra_details) {
        (Some(foo::ExtraDetails::OneOfStr(a)), Some(foo::ExtraDetails::OneOfStr(b))) => {
            assert_eq!(
                a.as_ref(),
                b.as_ref(),
                "extra_details OneOfStr variants do not match"
            );
        }
        (None, None) => {} // Both are None, which is fine
        _ => panic!("extra_details variants do not match"),
    }

    // === Additional Assertions for Cow::Owned in Decoded Instance (`g`) ===

    // Assert that `g.my_repeat` elements are `Cow::Owned`
    for (i, item_g) in g.my_repeat.iter().enumerate() {
        assert!(
            matches!(item_g, Cow::Owned(_)),
            "g.my_repeat[{}] should be Cow::Owned",
            i
        );
    }

    // Assert that `g.my_first_map` values are `Cow::Owned`
    for (key, val_g) in &g.my_first_map {
        assert!(
            matches!(val_g, Cow::Owned(_)),
            "g.my_first_map[{}] should be Cow::Owned",
            key
        );
    }

    // Assert that `g.my_second_map` keys are `Cow::Owned`
    for key_g in g.my_second_map.keys() {
        assert!(
            matches!(key_g, Cow::Owned(_)),
            "g.my_second_map key {:?} should be Cow::Owned",
            key_g
        );
    }

    // Assert that `g.my_third_map` keys and values are `Cow::Owned`
    for (key_g, val_g) in &g.my_third_map {
        assert!(
            matches!(key_g, Cow::Owned(_)),
            "g.my_third_map key {:?} should be Cow::Owned",
            key_g
        );
        assert!(
            matches!(val_g, Cow::Owned(_)),
            "g.my_third_map value {:?} should be Cow::Owned",
            val_g
        );
    }

    // Assert that `g.my_bytes` is `Cow::Owned`
    assert!(
        matches!(g.my_bytes, Cow::Owned(_)),
        "g.my_bytes should be Cow::Owned"
    );

    // Assert that `g.my_bytes_map` values are `Cow::Owned`
    for (key, val_g) in &g.my_bytes_map {
        assert!(
            matches!(val_g, Cow::Owned(_)),
            "g.my_bytes_map[{}] should be Cow::Owned",
            key
        );
    }

    // Assert that `g.my_vec_bytes` is `Cow::Owned`
    assert!(
        matches!(g.my_vec_bytes, Cow::Owned(_)),
        "g.my_vec_bytes should be Cow::Owned"
    );

    // Assert that `g.extra_details` variants are `Cow::Owned` if present
    if let Some(foo::ExtraDetails::OneOfStr(b)) = &g.extra_details {
        assert!(
            matches!(b, Cow::Owned(_)),
            "g.extra_details OneOfStr should be Cow::Owned"
        );
    }
}
