use std::{borrow::Cow, io::Write, ops::Deref, sync::Arc};

use anyhow::Result;
use async_recursion::async_recursion;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use ref_cast::RefCast;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sourcemap::{DecodedMap, SourceMap as RegularMap, SourceMapBuilder, SourceMapIndex};
use turbo_tasks::{RcStr, TryJoinIterExt, ValueToString, Vc};
use turbo_tasks_fs::{
    rope::{Rope, RopeBuilder},
    FileContent, FileSystemPath,
};

use crate::{source_pos::SourcePos, SOURCE_MAP_PREFIX};

pub(crate) mod source_map_asset;

pub use source_map_asset::SourceMapAsset;

/// Represents an empty value in a u32 variable in the sourcemap crate.
static SOURCEMAP_CRATE_NONE_U32: u32 = !0;

/// Allows callers to generate source maps.
#[turbo_tasks::value_trait]
pub trait GenerateSourceMap {
    /// Generates a usable source map, capable of both tracing and stringifying.
    fn generate_source_map(self: Vc<Self>) -> Vc<OptionSourceMap>;

    /// Returns an individual section of the larger source map, if found.
    fn by_section(self: Vc<Self>, _section: RcStr) -> Vc<OptionSourceMap> {
        Vc::cell(None)
    }
}

/// [SourceMap] enum implements the source map specification in 2 ways: A
/// "decoded" map which represents a source map as if it was came out of a JSON
/// decode, and a "sectioned" source map which is a tree of many [SourceMap]
/// covering regions of an output file.
///
/// The distinction between the source map spec's [sourcemap::Index] and our
/// [SourceMap::Sectioned] is whether the sections are represented with Vcs
/// pointers.
#[turbo_tasks::value(shared)]
pub enum SourceMap {
    /// A decoded source map contains no Vcs.
    Decoded(#[turbo_tasks(trace_ignore)] InnerSourceMap),
    /// A sectioned source map contains many (possibly recursive) maps covering
    /// different regions of the file.
    Sectioned(#[turbo_tasks(trace_ignore)] SectionedSourceMap),
}

#[turbo_tasks::value(transparent)]
pub struct SectionMapping(IndexMap<String, Vc<Box<dyn GenerateSourceMap>>>);

#[turbo_tasks::value(transparent)]
pub struct OptionSourceMap(Option<Vc<SourceMap>>);

#[turbo_tasks::value_impl]
impl OptionSourceMap {
    #[turbo_tasks::function]
    pub fn none() -> Vc<Self> {
        Vc::cell(None)
    }
}

#[turbo_tasks::value(transparent)]
#[derive(Clone, Debug)]
pub struct Tokens(Vec<Token>);

/// A token represents a mapping in a source map. It may either be Synthetic,
/// meaning it was generated by some build tool and doesn't represent a location
/// in a user-authored source file, or it is Original, meaning it represents a
/// real location in source file.
#[turbo_tasks::value]
#[derive(Clone, Debug)]
pub enum Token {
    Synthetic(SyntheticToken),
    Original(OriginalToken),
}

/// A SyntheticToken represents a region of the generated file that was created
/// by some build tool.
#[turbo_tasks::value]
#[derive(Clone, Debug)]
pub struct SyntheticToken {
    pub generated_line: usize,
    pub generated_column: usize,
    pub guessed_original_file: Option<String>,
}

/// An OriginalToken represents a region of the generated file that exists in
/// user-authored source file.
#[turbo_tasks::value]
#[derive(Clone, Debug)]
pub struct OriginalToken {
    pub generated_line: usize,
    pub generated_column: usize,
    pub original_file: String,
    pub original_line: usize,
    pub original_column: usize,
    pub name: Option<RcStr>,
}

impl Token {
    pub fn generated_line(&self) -> usize {
        match self {
            Self::Original(t) => t.generated_line,
            Self::Synthetic(t) => t.generated_line,
        }
    }

    pub fn generated_column(&self) -> usize {
        match self {
            Self::Original(t) => t.generated_column,
            Self::Synthetic(t) => t.generated_column,
        }
    }
}

impl<'a> From<sourcemap::Token<'a>> for Token {
    fn from(t: sourcemap::Token) -> Self {
        if t.has_source() {
            Token::Original(OriginalToken {
                generated_line: t.get_dst_line() as usize,
                generated_column: t.get_dst_col() as usize,
                original_file: t
                    .get_source()
                    .expect("already checked token has source")
                    .to_string(),
                original_line: t.get_src_line() as usize,
                original_column: t.get_src_col() as usize,
                name: t.get_name().map(String::from),
            })
        } else {
            Token::Synthetic(SyntheticToken {
                generated_line: t.get_dst_line() as usize,
                generated_column: t.get_dst_col() as usize,
                guessed_original_file: None,
            })
        }
    }
}

impl TryInto<sourcemap::RawToken> for Token {
    type Error = std::num::ParseIntError;

    fn try_into(self) -> Result<sourcemap::RawToken, Self::Error> {
        Ok(match self {
            Self::Original(t) => sourcemap::RawToken {
                dst_col: t.generated_column as u32,
                dst_line: t.generated_line as u32,
                name_id: match t.name {
                    None => SOURCEMAP_CRATE_NONE_U32,
                    Some(name) => name.parse()?,
                },
                src_col: t.original_column as u32,
                src_line: t.original_line as u32,
                src_id: t.original_file.parse()?,
                is_range: false,
            },
            Self::Synthetic(t) => sourcemap::RawToken {
                dst_col: t.generated_column as u32,
                dst_line: t.generated_line as u32,
                name_id: SOURCEMAP_CRATE_NONE_U32,
                src_col: SOURCEMAP_CRATE_NONE_U32,
                src_line: SOURCEMAP_CRATE_NONE_U32,
                src_id: SOURCEMAP_CRATE_NONE_U32,
                is_range: false,
            },
        })
    }
}

impl SourceMap {
    /// Creates a new SourceMap::Decoded Vc out of a [RegularMap] instance.
    pub fn new_regular(map: RegularMap) -> Self {
        Self::new_decoded(DecodedMap::Regular(map))
    }

    /// Creates a new SourceMap::Decoded Vc out of a [DecodedMap] instance.
    pub fn new_decoded(map: DecodedMap) -> Self {
        SourceMap::Decoded(InnerSourceMap::new(map))
    }

    /// Creates a new SourceMap::Sectioned Vc out of a collection of source map
    /// sections.
    pub fn new_sectioned(sections: Vec<SourceMapSection>) -> Self {
        SourceMap::Sectioned(SectionedSourceMap::new(sections))
    }

    pub async fn new_from_file(file: Vc<FileSystemPath>) -> Result<Option<Self>> {
        let read = file.read();
        Self::new_from_file_content(read).await
    }

    pub async fn new_from_file_content(content: Vc<FileContent>) -> Result<Option<Self>> {
        let content = &content.await?;
        let Some(contents) = content.as_content() else {
            return Ok(None);
        };
        let Ok(map) = DecodedMap::from_reader(contents.read()) else {
            return Ok(None);
        };
        Ok(Some(SourceMap::Decoded(InnerSourceMap::new(map))))
    }
}

impl SourceMap {
    pub async fn to_source_map(&self) -> Result<Arc<CrateMapWrapper>> {
        Ok(match self {
            Self::Decoded(m) => m.map.clone(),
            Self::Sectioned(m) => {
                let wrapped = m.to_crate_wrapper().await?;
                let sections = wrapped
                    .sections
                    .iter()
                    .map(|s| {
                        sourcemap::SourceMapSection::new(
                            (s.offset.line as u32, s.offset.column as u32),
                            None,
                            Some(s.map.0.clone()),
                        )
                    })
                    .collect::<Vec<sourcemap::SourceMapSection>>();
                Arc::new(CrateMapWrapper(DecodedMap::Index(SourceMapIndex::new(
                    None, sections,
                ))))
            }
        })
    }
}

#[turbo_tasks::value_impl]
impl SourceMap {
    /// A source map that contains no actual source location information (no
    /// `sources`, no mappings that point into a source). This is used to tell
    /// Chrome that the generated code starting at a particular offset is no
    /// longer part of the previous section's mappings.
    #[turbo_tasks::function]
    pub fn empty() -> Vc<Self> {
        let mut builder = SourceMapBuilder::new(None);
        builder.add(0, 0, 0, 0, None, None, false);
        SourceMap::new_regular(builder.into_sourcemap()).cell()
    }

    /// Stringifies the source map into JSON bytes.
    #[turbo_tasks::function]
    pub async fn to_rope(self: Vc<Self>) -> Result<Vc<Rope>> {
        let this = self.await?;
        let rope = match &*this {
            SourceMap::Decoded(r) => {
                let mut bytes = vec![];
                r.0.to_writer(&mut bytes)?;
                Rope::from(bytes)
            }

            SourceMap::Sectioned(s) => {
                if s.sections.len() == 1 {
                    let s = &s.sections[0];
                    if s.offset == (0, 0) {
                        return Ok(s.map.to_rope());
                    }
                }

                // My kingdom for a decent dedent macro with interpolation!
                // NOTE: The empty `sources` array is technically incorrect, but there is a bug
                // in Node.js that requires sectioned source maps to have a `sources` array.
                let mut rope = RopeBuilder::from(
                    r#"{
      "version": 3,
      "sources": [],
      "sections": ["#,
                );

                let sections = s
                    .sections
                    .iter()
                    .map(|s| async move { Ok((s.offset, s.map.to_rope().await?)) })
                    .try_join()
                    .await?;

                let mut first_section = true;
                for (offset, section_map) in sections {
                    if !first_section {
                        rope += ",";
                    }
                    first_section = false;

                    write!(
                        rope,
                        r#"
    {{"offset": {{"line": {}, "column": {}}}, "map": "#,
                        offset.line, offset.column,
                    )?;

                    rope += &*section_map;

                    rope += "}";
                }

                rope += "]
    }";

                rope.build()
            }
        };
        Ok(rope.cell())
    }

    /// Traces a generated line/column into an mapping token representing either
    /// synthetic code or user-authored original code.
    #[turbo_tasks::function]
    pub async fn lookup_token(self: Vc<Self>, line: usize, column: usize) -> Result<Vc<Token>> {
        let token = match &*self.await? {
            SourceMap::Decoded(map) => {
                let mut token = map
                    .lookup_token(line as u32, column as u32)
                    .map(Token::from)
                    .unwrap_or_else(|| {
                        Token::Synthetic(SyntheticToken {
                            generated_line: line,
                            generated_column: column,
                            guessed_original_file: None,
                        })
                    });
                if let Token::Synthetic(SyntheticToken {
                    guessed_original_file,
                    ..
                }) = &mut token
                {
                    if let DecodedMap::Regular(map) = &map.map.0 {
                        if map.get_source_count() == 1 {
                            let source = map.sources().next().unwrap();
                            *guessed_original_file = Some(source.to_string());
                        }
                    }
                }
                token
            }

            SourceMap::Sectioned(map) => {
                let len = map.sections.len();
                let mut low = 0;
                let mut high = len;
                let pos = SourcePos { line, column };

                // A "greatest lower bound" binary search. We're looking for the closest section
                // offset <= to our line/col.
                while low < high {
                    let mid = (low + high) / 2;
                    if pos < map.sections[mid].offset {
                        high = mid;
                    } else {
                        low = mid + 1;
                    }
                }

                // Our GLB search will return the section immediately to the right of the
                // section we actually want to recurse into, because the binary search does not
                // early exit on an exact match (it'll `low = mid + 1`).
                if low > 0 && low <= len {
                    let SourceMapSection { map, offset } = &map.sections[low - 1];
                    // We're looking for the position `l` lines into region covered by this
                    // sourcemap's section.
                    let l = line - offset.line;
                    // The source map starts offset by the section's column only on its first line.
                    // On the 2nd+ line, the source map covers starting at column 0.
                    let c = if line == offset.line {
                        column - offset.column
                    } else {
                        column
                    };
                    return Ok(map.lookup_token(l, c));
                }
                Token::Synthetic(SyntheticToken {
                    generated_line: line,
                    generated_column: column,
                    guessed_original_file: None,
                })
            }
        };
        Ok(token.cell())
    }

    #[turbo_tasks::function]
    pub async fn with_resolved_sources(
        self: Vc<Self>,
        origin: Vc<FileSystemPath>,
    ) -> Result<Vc<Self>> {
        async fn resolve_source(
            source_request: Arc<str>,
            source_content: Option<Arc<str>>,
            origin: Vc<FileSystemPath>,
        ) -> Result<(Arc<str>, Arc<str>)> {
            Ok(
                if let Some(path) = *origin.parent().try_join((&*source_request).into()).await? {
                    let path_str = path.to_string().await?;
                    let source = format!("{SOURCE_MAP_PREFIX}{}", path_str);
                    let source_content = if let Some(source_content) = source_content {
                        source_content
                    } else if let FileContent::Content(file) = &*path.read().await? {
                        let text = file.content().to_str()?;
                        text.to_string().into()
                    } else {
                        format!("unable to read source {path_str}").into()
                    };
                    (source.into(), source_content)
                } else {
                    let origin_str = origin.to_string().await?;
                    static INVALID_REGEX: Lazy<Regex> =
                        Lazy::new(|| Regex::new(r#"(?:^|/)(?:\.\.?(?:/|$))+"#).unwrap());
                    let source = INVALID_REGEX
                        .replace_all(&source_request, |s: &regex::Captures<'_>| {
                            s[0].replace('.', "_")
                        });
                    let source = format!("{SOURCE_MAP_PREFIX}{}/{}", origin_str, source);
                    let source_content = source_content.unwrap_or_else(|| {
                        format!(
                            "unable to access {source_request} in {origin_str} (it's leaving the \
                             filesystem root)"
                        )
                        .into()
                    });
                    (source.into(), source_content)
                },
            )
        }
        async fn regular_map_with_resolved_sources(
            map: &RegularMapWrapper,
            origin: Vc<FileSystemPath>,
        ) -> Result<RegularMap> {
            let map = &map.0;
            let file = map.get_file().map(Arc::<str>::from);
            let tokens = map.tokens().map(|t| t.get_raw_token()).collect();
            let names = map.names().map(Arc::<str>::from).collect();
            let count = map.get_source_count() as usize;
            let sources = map.sources().map(Arc::<str>::from).collect::<Vec<_>>();
            let source_contents = map
                .source_contents()
                .map(|s| s.map(Arc::<str>::from))
                .collect::<Vec<_>>();
            let mut new_sources = Vec::with_capacity(count);
            let mut new_source_contents = Vec::with_capacity(count);
            for (source, source_content) in sources.into_iter().zip(source_contents.into_iter()) {
                let (source, name) = resolve_source(source, source_content, origin).await?;
                new_sources.push(source);
                new_source_contents.push(Some(name));
            }
            Ok(RegularMap::new(
                file,
                tokens,
                names,
                new_sources,
                Some(new_source_contents),
            ))
        }
        #[async_recursion]
        async fn decoded_map_with_resolved_sources(
            map: &CrateMapWrapper,
            origin: Vc<FileSystemPath>,
        ) -> Result<CrateMapWrapper> {
            Ok(CrateMapWrapper(match &map.0 {
                DecodedMap::Regular(map) => {
                    let map = RegularMapWrapper::ref_cast(map);
                    DecodedMap::Regular(regular_map_with_resolved_sources(map, origin).await?)
                }
                DecodedMap::Index(map) => {
                    let count = map.get_section_count() as usize;
                    let file = map.get_file().map(ToString::to_string);
                    let sections = map
                        .sections()
                        .filter_map(|section| {
                            section
                                .get_sourcemap()
                                .map(|s| (section.get_offset(), CrateMapWrapper::ref_cast(s)))
                        })
                        .collect::<Vec<_>>();
                    let sections = sections
                        .into_iter()
                        .map(|(offset, map)| async move {
                            Ok((
                                offset,
                                decoded_map_with_resolved_sources(map, origin).await?,
                            ))
                        })
                        .try_join()
                        .await?;
                    let mut new_sections = Vec::with_capacity(count);
                    for (offset, map) in sections {
                        new_sections.push(sourcemap::SourceMapSection::new(
                            offset,
                            // Urls are deprecated and we don't accept them
                            None,
                            Some(map.0),
                        ));
                    }
                    DecodedMap::Index(SourceMapIndex::new(file, new_sections))
                }
                DecodedMap::Hermes(_) => {
                    todo!("hermes source maps are not implemented");
                }
            }))
        }
        Ok(match &*self.await? {
            Self::Decoded(m) => {
                let map = decoded_map_with_resolved_sources(&m.map, origin).await?;
                Self::Decoded(InnerSourceMap::new(map.0))
            }
            Self::Sectioned(m) => {
                let mut sections = Vec::with_capacity(m.sections.len());
                for section in &m.sections {
                    let map = section.map.with_resolved_sources(origin);
                    sections.push(SourceMapSection::new(section.offset, map));
                }
                for section in &mut sections {
                    section.map = section.map.resolve().await?;
                }
                SourceMap::new_sectioned(sections)
            }
        }
        .cell())
    }
}

#[turbo_tasks::value_impl]
impl GenerateSourceMap for SourceMap {
    #[turbo_tasks::function]
    fn generate_source_map(self: Vc<Self>) -> Vc<OptionSourceMap> {
        Vc::cell(Some(self))
    }

    #[turbo_tasks::function]
    fn by_section(&self, _section: RcStr) -> Vc<OptionSourceMap> {
        Vc::cell(None)
    }
}

/// A regular source map covers an entire file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerSourceMap {
    map: Arc<CrateMapWrapper>,
}

impl InnerSourceMap {
    pub fn new(map: DecodedMap) -> Self {
        InnerSourceMap {
            map: Arc::new(CrateMapWrapper(map)),
        }
    }
}

impl Deref for InnerSourceMap {
    type Target = Arc<CrateMapWrapper>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl Eq for InnerSourceMap {}
impl PartialEq for InnerSourceMap {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.map, &other.map)
    }
}

/// Wraps the DecodedMap struct so that it is Sync and Send.
///
/// # Safety
///
/// Must not use per line access to the SourceMap, as it is not thread safe.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct CrateMapWrapper(DecodedMap);

// Safety: DecodedMap contains a raw pointer, which isn't Send, which is
// required to cache in a Vc. So, we have wrap it in 4 layers of cruft to do it.
unsafe impl Send for CrateMapWrapper {}
unsafe impl Sync for CrateMapWrapper {}

/// Wraps the RegularMap struct so that it is Sync and Send.
///
/// # Safety
///
/// Must not use per line access to the SourceMap, as it is not thread safe.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct RegularMapWrapper(RegularMap);

// Safety: RegularMap contains a raw pointer, which isn't Send, which is
// required to cache in a Vc. So, we have wrap it in 4 layers of cruft to do it.
unsafe impl Send for RegularMapWrapper {}
unsafe impl Sync for RegularMapWrapper {}

#[derive(Debug)]
pub struct CrateIndexWrapper {
    pub sections: Vec<CrateSectionWrapper>,
}

#[derive(Debug)]
pub struct CrateSectionWrapper {
    pub offset: SourcePos,
    pub map: Arc<CrateMapWrapper>,
}

impl CrateMapWrapper {
    pub fn as_regular_source_map(&self) -> Option<Cow<'_, RegularMap>> {
        match &self.0 {
            DecodedMap::Regular(m) => Some(Cow::Borrowed(m)),
            DecodedMap::Index(m) => m.flatten().map(Cow::Owned).ok(),
            _ => None,
        }
    }
}

impl Deref for CrateMapWrapper {
    type Target = DecodedMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for CrateMapWrapper {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        let mut bytes = vec![];
        self.0.to_writer(&mut bytes).map_err(Error::custom)?;
        serializer.serialize_bytes(bytes.as_slice())
    }
}

impl<'de> Deserialize<'de> for CrateMapWrapper {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let bytes = <&[u8]>::deserialize(deserializer)?;
        let map = DecodedMap::from_reader(bytes).map_err(Error::custom)?;
        Ok(CrateMapWrapper(map))
    }
}

/// A sectioned source map contains many (possibly recursive) maps covering
/// different regions of the file.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct SectionedSourceMap {
    sections: Vec<SourceMapSection>,
}

impl SectionedSourceMap {
    pub fn new(sections: Vec<SourceMapSection>) -> Self {
        Self { sections }
    }

    pub async fn to_crate_wrapper(&self) -> Result<CrateIndexWrapper> {
        let mut sections = Vec::with_capacity(self.sections.len());
        for section in &self.sections {
            sections.push(section.to_crate_wrapper().await?);
        }
        Ok(CrateIndexWrapper { sections })
    }
}

/// A section of a larger sectioned source map, which applies at source
/// positions >= the offset (until the next section starts).
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct SourceMapSection {
    offset: SourcePos,
    map: Vc<SourceMap>,
}

impl SourceMapSection {
    pub fn new(offset: SourcePos, map: Vc<SourceMap>) -> Self {
        Self { offset, map }
    }

    #[async_recursion]
    pub async fn to_crate_wrapper(&self) -> Result<CrateSectionWrapper> {
        let map = (*self.map.await?).to_source_map().await?;
        Ok(CrateSectionWrapper {
            offset: self.offset,
            map,
        })
    }
}
