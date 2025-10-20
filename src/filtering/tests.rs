use super::*;

#[test]
fn test_filter() {
    let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::<DefaultNormalizing>::default());
    filter.entry("a").exclude();
    filter.entry("a.b").include();
    filter.entry("c.d").include();
    filter.entry("c.d.e").exclude();
    filter.entry("c.d.g*").exclude();

    assert!(filter.get("x").is_none());

    let a = filter.get("a").unwrap();
    assert_eq!(a.setting(), IncludeExcludeSetting::Exclude);

    let ab = a.get("b").unwrap();
    assert_eq!(ab.setting(), IncludeExcludeSetting::Include);

    let ab = filter.get("a.b").unwrap();
    assert_eq!(ab.setting(), IncludeExcludeSetting::Include);

    let ac = filter.get("a.c").unwrap();
    assert_eq!(ac.setting(), IncludeExcludeSetting::Exclude);

    let c = filter.get("c").unwrap();
    assert_eq!(c.setting(), IncludeExcludeSetting::Unspecified);

    let cd = c.get("d").unwrap();
    assert_eq!(cd.setting(), IncludeExcludeSetting::Include);

    let cd = filter.get("c.d").unwrap();
    assert_eq!(cd.setting(), IncludeExcludeSetting::Include);

    assert!(c.get("e").is_none());
    assert!(filter.get("c.e").is_none());

    let cde = cd.get("e").unwrap();
    assert_eq!(cde.setting(), IncludeExcludeSetting::Exclude);

    let cde = filter.get("c.d.e").unwrap();
    assert_eq!(cde.setting(), IncludeExcludeSetting::Exclude);

    let cde = filter.get("c.d.e").unwrap();
    assert_eq!(cde.setting(), IncludeExcludeSetting::Exclude);

    let cdf = cd.get("f").unwrap();
    assert_eq!(cdf.setting(), IncludeExcludeSetting::Include);

    let cdf = filter.get("c.d.f").unwrap();
    assert_eq!(cdf.setting(), IncludeExcludeSetting::Include);

    let cdef = filter.get("c.d.e.f").unwrap();
    assert_eq!(cdef.setting(), IncludeExcludeSetting::Exclude);

    let cdg = filter.get("c.d.g").unwrap();
    assert_eq!(cdg.setting(), IncludeExcludeSetting::Exclude);

    let cdg2 = filter.get("c.d.g2").unwrap();
    assert_eq!(cdg2.setting(), IncludeExcludeSetting::Exclude);

    let filter = filter.excluded();

    assert_eq!(filter.get("x").unwrap().setting(), IncludeExcludeSetting::Exclude);

    let a = filter.get("a").unwrap();
    assert_eq!(a.setting(), IncludeExcludeSetting::Exclude);

    let ab = a.get("b").unwrap();
    assert_eq!(ab.setting(), IncludeExcludeSetting::Exclude);

    let ab = filter.get("a.b").unwrap();
    assert_eq!(ab.setting(), IncludeExcludeSetting::Exclude);

    let ac = filter.get("a.c").unwrap();
    assert_eq!(ac.setting(), IncludeExcludeSetting::Exclude);

    let c = filter.get("c").unwrap();
    assert_eq!(c.setting(), IncludeExcludeSetting::Exclude);

    let cd = c.get("d").unwrap();
    assert_eq!(cd.setting(), IncludeExcludeSetting::Exclude);

    let cd = filter.get("c.d").unwrap();
    assert_eq!(cd.setting(), IncludeExcludeSetting::Exclude);

    assert_eq!(c.get("e").unwrap().setting(), IncludeExcludeSetting::Exclude);
    assert_eq!(filter.get("c.e").unwrap().setting(), IncludeExcludeSetting::Exclude);

    let cde = cd.get("e").unwrap();
    assert_eq!(cde.setting(), IncludeExcludeSetting::Exclude);

    let cde = filter.get("c.d.e").unwrap();
    assert_eq!(cde.setting(), IncludeExcludeSetting::Exclude);

    let cde = filter.get("c.d.e").unwrap();
    assert_eq!(cde.setting(), IncludeExcludeSetting::Exclude);

    let cdf = cd.get("f").unwrap();
    assert_eq!(cdf.setting(), IncludeExcludeSetting::Exclude);

    let cdf = filter.get("c.d.f").unwrap();
    assert_eq!(cdf.setting(), IncludeExcludeSetting::Exclude);

    let cdef = filter.get("c.d.e.f").unwrap();
    assert_eq!(cdef.setting(), IncludeExcludeSetting::Exclude);

    let cdg = filter.get("c.d.g").unwrap();
    assert_eq!(cdg.setting(), IncludeExcludeSetting::Exclude);

    let cdg2 = filter.get("c.d.g2").unwrap();
    assert_eq!(cdg2.setting(), IncludeExcludeSetting::Exclude);

    let filter = filter.included();

    assert_eq!(filter.get("x").unwrap().setting(), IncludeExcludeSetting::Include);
}
