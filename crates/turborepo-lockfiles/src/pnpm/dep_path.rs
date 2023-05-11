use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    combinator::{opt, recognize},
    sequence::tuple,
    Finish, IResult,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DepPath<'a> {
    // todo we possibly keep the full string here for 0-cost serialization
    pub name: &'a str,
    pub version: &'a str,
    pub host: Option<&'a str>,
    pub peer_suffix: Option<&'a str>,
}

impl<'a> DepPath<'a> {
    pub fn new(name: &'a str, version: &'a str) -> Self {
        Self {
            name,
            version,
            host: None,
            peer_suffix: None,
        }
    }

    pub fn with_host(mut self, host: Option<&'a str>) -> Self {
        self.host = host;
        self
    }

    pub fn with_peer_suffix(mut self, peer_suffix: Option<&'a str>) -> Self {
        self.peer_suffix = peer_suffix;
        self
    }

    pub fn patch_hash(&self) -> Option<&str> {
        self.peer_suffix.and_then(|s| {
            if s.starts_with('(') {
                let (_, suffixes) = parse_v6_suffixes(s).ok()?;
                suffixes.iter().find_map(|s| s.strip_prefix("patch_hash="))
            // Check if suffix is pre v6
            } else if let Some(idx) = s.find('_') {
                Some(&s[..idx])
            } else {
                // if a dependency just has a single suffix we can't tell if it's
                // a patch or peer hash return it in case it's a patch hash
                Some(s)
            }
        })
    }
}

impl<'a> TryFrom<&'a str> for DepPath<'a> {
    type Error = nom::error::Error<String>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (_, dep_path) = parse_dep_path(value).map_err(|e| e.to_owned()).finish()?;
        Ok(dep_path)
    }
}

// See https://github.com/pnpm/pnpm/blob/185ab01adfc927ea23d2db08a14723bf51d0025f/packages/dependency-path/src/index.ts#L96
fn parse_dep_path(i: &str) -> IResult<&str, DepPath> {
    let (i, host) = parse_host(i)?;
    let (i, _) = nom::character::complete::char('/')(i)?;
    let (i, name) = parse_name(i)?;
    let (i, _) = nom::character::complete::one_of("/@")(i)?;
    let (i, version) = parse_version(i)?;
    let (i, peer_suffix) = opt(alt((parse_new_peer_suffix, parse_old_peer_suffix)))(i)?;
    Ok((
        i,
        DepPath::new(name, version)
            .with_host(host)
            .with_peer_suffix(peer_suffix),
    ))
}

fn parse_host(i: &str) -> IResult<&str, Option<&str>> {
    let (i, host) = opt(is_not("/"))(i)?;
    Ok((i, host))
}

fn parse_name(i: &str) -> IResult<&str, &str> {
    let (i, name) = alt((parse_name_with_scope, is_not("/@")))(i)?;
    Ok((i, name))
}

fn parse_name_with_scope(i: &str) -> IResult<&str, &str> {
    let (i, name) = recognize(tuple((tag("@"), is_not("/"), tag("/"), is_not("/@"))))(i)?;
    Ok((i, name))
}

fn parse_version(i: &str) -> IResult<&str, &str> {
    // pre v6 lockfiles use _ to delimit version from metadata
    // v6+ wraps metadata in (
    let (i, version) = is_not("_(")(i)?;
    Ok((i, version))
}

fn parse_old_peer_suffix(i: &str) -> IResult<&str, &str> {
    let (rest, _) = tag("_")(i)?;
    Ok(("", rest))
}

fn parse_new_peer_suffix(i: &str) -> IResult<&str, &str> {
    let (i, suffix) = recognize(parse_v6_suffixes)(i)?;
    Ok((i, suffix))
}

fn parse_v6_suffix(i: &str) -> IResult<&str, &str> {
    let (i, _) = tag("(")(i)?;
    let (i, entry) = is_not(")")(i)?;
    let (i, _) = tag(")")(i)?;
    Ok((i, entry))
}

fn parse_v6_suffixes(i: &str) -> IResult<&str, Vec<&str>> {
    let (i, suffixes) = nom::multi::many1(parse_v6_suffix)(i)?;
    Ok((i, suffixes))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test_case("/foo/1.0.0", DepPath::new("foo", "1.0.0"); "basic dep path")]
    #[test_case("/@foo/bar/1.0.0", DepPath::new("@foo/bar", "1.0.0"); "scoped dep path")]
    #[test_case("example.org/foo/1.0.0", DepPath::new("foo", "1.0.0").with_host(Some("example.org")); "dep path with custom host")]
    #[test_case("/foo/1.0.0_bar@1.0.0", DepPath::new("foo", "1.0.0").with_peer_suffix(Some("bar@1.0.0")); "dep path with peer dependency")]
    #[test_case("/foo/1.0.0(bar@1.0.0)", DepPath::new("foo", "1.0.0").with_peer_suffix(Some("(bar@1.0.0)")); "dep path with new peer dependency")]
    #[test_case("/foo/1.0.0_patchHash_peerHash", DepPath::new("foo", "1.0.0").with_peer_suffix(Some("patchHash_peerHash")); "dep path with path and peer hash")]
    #[test_case("/@babel/helper-string-parser/7.19.4(patch_hash=wjhgmpzh47qmycrzgpeyoyh3ce)(@babel/core@7.21.0)", DepPath::new("@babel/helper-string-parser", "7.19.4").with_peer_suffix(Some("(patch_hash=wjhgmpzh47qmycrzgpeyoyh3ce)(@babel/core@7.21.0)")); "dep path with new path and peer hash")]
    #[test_case("/foo@1.0.0", DepPath::new("foo", "1.0.0"); "basic v6 dep path")]
    #[test_case("/is-even@1.0.0_foobar", DepPath::new("is-even", "1.0.0").with_peer_suffix(Some("foobar")); "v6 dep path with suffix")]
    #[test_case("/foo@1.0.0(bar@1.0.0)(baz@1.0.0)", DepPath::new("foo", "1.0.0").with_peer_suffix(Some("(bar@1.0.0)(baz@1.0.0)")); "v6 with multiple peers")]
    #[test_case("/@babel/helper-string-parser@7.19.4(patch_hash=wjhgmpzh47qmycrzgpeyoyh3ce)(@babel/core@7.21.0)", DepPath::new("@babel/helper-string-parser", "7.19.4").with_peer_suffix(Some("(patch_hash=wjhgmpzh47qmycrzgpeyoyh3ce)(@babel/core@7.21.0)")); "v6 with scope")]
    fn dep_path_parse_tests(s: &str, expected: DepPath) {
        let (rest, actual) = parse_dep_path(s).unwrap();
        assert_eq!(rest, "");
        assert_eq!(actual, expected);
    }

    #[test_case("/@babel/helper-string-parser/7.19.4(patch_hash=wjhgmpzh47qmycrzgpeyoyh3ce)(@babel/core@7.21.0)", Some("wjhgmpzh47qmycrzgpeyoyh3ce"); "v6 patch")]
    #[test_case("/foo/1.0.0_patchHash_peerHash", Some("patchHash"); "pre v6 patch")]
    #[test_case("/foo/1.0.0", None; "no suffix")]
    #[test_case("/foo/1.0.0(bar@1.0.0)", None; "no patch")]
    fn dep_path_patch_hash(input: &str, expected: Option<&str>) {
        let dep_path = DepPath::try_from(input).unwrap();
        assert_eq!(dep_path.patch_hash(), expected);
    }
}
