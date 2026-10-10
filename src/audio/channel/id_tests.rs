use super::Id;
use strum::IntoEnumIterator;

#[test]
fn short_names_preserve_workplace_spelling() {
    assert_eq!(Id::Left.short_name(), "L");
    assert_eq!(Id::Right.short_name(), "R");
    assert_eq!(Id::Center.short_name(), "C");
    assert_eq!(Id::LFE.short_name(), "LFE");
    assert_eq!(Id::Mono.short_name(), "M");
    assert_eq!(Id::LeftCenter.short_name(), "LC");
    assert_eq!(Id::RightCenter.short_name(), "RC");
}

#[test]
fn names_and_parsing_preserve_the_existing_api() {
    assert_eq!(Id::Left.long_name(), "Left");
    assert_eq!(Id::LFE.long_name(), "LFE");
    assert_eq!(Id::Mono.long_name(), "Mono");
    assert_eq!(Id::LeftCenter.as_ref(), "LeftCenter");
    assert_eq!("LeftCenter".parse::<Id>().unwrap(), Id::LeftCenter);
    assert_eq!("LFE".parse::<Id>().unwrap(), Id::LFE);
    assert!("UnknownChannel".parse::<Id>().is_err());
}

#[test]
fn iteration_and_explicit_values_remain_stable() {
    let variants: Vec<_> = Id::iter().collect();
    assert_eq!(variants.first(), Some(&Id::Left));
    assert!(variants.contains(&Id::Mono));
    assert!(
        variants
            .windows(2)
            .all(|pair| (pair[0] as u32) < pair[1] as u32)
    );
    assert_eq!(Id::Left as u32, 0);
    assert_eq!(Id::LeftCenter as u32, 18);
    assert_eq!(Id::RightCenter as u32, 19);
    assert_eq!(Id::Mono as u32, 30);
}
