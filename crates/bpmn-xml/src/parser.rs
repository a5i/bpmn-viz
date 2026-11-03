//! Namespace-aware streaming importer for BPMN 2.0 XML.
//!
//! Safety properties (SPEC-001 section 4):
//! - bounded input size and nesting depth,
//! - subprocess nesting is parsed iteratively, so depth never maps to native recursion,
//! - no entity expansion (custom entities are rejected as syntax errors),
//! - no external resource fetching,
//! - unknown elements are skipped, never crash parsing.

use crate::namespaces::{BPMN_DI, BPMN_MODEL, DC, DI, XSI};
use bpmn_model::di::{Bounds, BpmnLabel, Diagram, Edge, Plane, Point, Shape};
use bpmn_model::element::{
    Artifact, Collaboration, Definitions, EventDefinition, FlowElement, FlowNode, GatewayType,
    Lane, LaneSet, MessageFlow, NodeKind, Participant, Process, RootElement, SequenceFlow,
    TaskType, TextAnnotation, UnsupportedElement,
};
use bpmn_model::{BpmnDocument, Diagnostic, DiagnosticCode, SourceLocation};
use quick_xml::NsReader;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{QName, ResolveResult};

/// Import tuning and safety limits.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Reject larger inputs before parsing (default 64 MiB).
    pub max_bytes: usize,
    /// Maximum element nesting depth (default 512).
    pub max_depth: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_depth: 512,
        }
    }
}

/// Fatal import failure carrying one structured diagnostic.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The diagnostic that aborted import.
    pub diagnostic: Diagnostic,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diagnostic)
    }
}

impl std::error::Error for ParseError {}

/// Parses BPMN 2.0 XML bytes with default [`ParseOptions`].
pub fn from_xml(bytes: &[u8]) -> Result<BpmnDocument, ParseError> {
    from_xml_with_options(bytes, &ParseOptions::default())
}

/// Parses BPMN 2.0 XML bytes with explicit options.
pub fn from_xml_with_options(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<BpmnDocument, ParseError> {
    if bytes.len() > options.max_bytes {
        return Err(ParseError {
            diagnostic: Diagnostic::error(
                DiagnosticCode::XmlSizeLimit,
                format!(
                    "input is {} bytes, exceeding the configured limit of {} bytes",
                    bytes.len(),
                    options.max_bytes
                ),
            ),
        });
    }
    let text = decode_utf8(bytes)?;
    let mut parser = Parser::new(text, options.max_depth);
    parser.parse_document()?;
    if parser.ignored_elements > 0 {
        parser.diagnostics.push(Diagnostic::warning(
            DiagnosticCode::IgnoredElement,
            format!(
                "{} unsupported or unknown element(s) were ignored during import",
                parser.ignored_elements
            ),
        ));
    }
    parser.definitions.unsupported = parser.unsupported;
    Ok(BpmnDocument {
        definitions: parser.definitions,
        parse_diagnostics: parser.diagnostics,
    })
}

fn decode_utf8(bytes: &[u8]) -> Result<&str, ParseError> {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return Err(ParseError {
            diagnostic: Diagnostic::error(
                DiagnosticCode::XmlEncoding,
                "UTF-16 input is not supported; convert the document to UTF-8",
            ),
        });
    }
    // Tolerate a UTF-8 BOM.
    let bytes = bytes
        .strip_prefix([0xEF, 0xBB, 0xBF].as_slice())
        .unwrap_or(bytes);
    std::str::from_utf8(bytes).map_err(|error| ParseError {
        diagnostic: Diagnostic::error(
            DiagnosticCode::XmlEncoding,
            format!("input is not valid UTF-8: {error}"),
        ),
    })
}

/// Resolved name of an element: namespace URI plus local name.
struct ResolvedName {
    namespace: Option<String>,
    local: String,
}

impl ResolvedName {
    fn matches(&self, namespace: &str, local: &str) -> bool {
        self.namespace.as_deref() == Some(namespace) && self.local == local
    }
}

/// Incremental streaming parser state.
struct Parser<'a> {
    reader: NsReader<&'a [u8]>,
    source: &'a str,
    diagnostics: Vec<Diagnostic>,
    definitions: Definitions,
    depth: usize,
    max_depth: usize,
    ignored_elements: usize,
    /// Elements modeled only for id/DI resolution, in source order.
    unsupported: Vec<UnsupportedElement>,
}

/// Per-node content captured from children.
#[derive(Default)]
struct NodeContent {
    incoming: Vec<String>,
    outgoing: Vec<String>,
    event_definition: EventDefinition,
    default_flow_ref: Option<String>,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, max_depth: usize) -> Self {
        let mut reader = NsReader::from_str(text);
        reader.config_mut().trim_text(true);
        Self {
            reader,
            source: text,
            diagnostics: Vec::new(),
            definitions: Definitions::default(),
            depth: 0,
            max_depth,
            ignored_elements: 0,
            unsupported: Vec::new(),
        }
    }

    fn position(&self) -> SourceLocation {
        let offset = (self.reader.buffer_position() as usize).min(self.source.len());
        let before = &self.source[..offset];
        let line = before.matches('\n').count() + 1;
        let column = before.chars().rev().take_while(|&c| c != '\n').count() + 1;
        SourceLocation { line, column }
    }

    fn syntax_error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            diagnostic: Diagnostic::error(DiagnosticCode::XmlSyntax, message)
                .with_location(self.position()),
        }
    }

    fn resolve_name(&mut self, raw: QName<'_>) -> ResolvedName {
        let (resolve, local) = self.reader.resolver().resolve_element(raw);
        match resolve {
            ResolveResult::Bound(namespace) => ResolvedName {
                namespace: Some(namespace.0.to_string()),
                local: local.into_inner().to_string(),
            },
            ResolveResult::Unbound => ResolvedName {
                namespace: None,
                local: local.into_inner().to_string(),
            },
            ResolveResult::Unknown(prefix) => {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::XmlUnresolvedPrefix,
                        format!("namespace prefix `{prefix}` is not declared; element ignored"),
                    )
                    .with_location(self.position()),
                );
                ResolvedName {
                    namespace: None,
                    local: local.into_inner().to_string(),
                }
            }
        }
    }

    /// Non-xmlns, no-namespace attributes plus an optional `xsi:type` local
    /// name (prefix resolved against in-scope declarations).
    fn attributes(&mut self, start: &BytesStart<'_>) -> (Vec<(String, String)>, Option<String>) {
        let mut plain = Vec::new();
        let mut xsi_type = None;
        for attribute in start.attributes() {
            let Ok(attribute) = attribute else {
                continue;
            };
            let raw_key = attribute.key.into_inner();
            if raw_key == "xmlns" || raw_key.starts_with("xmlns:") {
                continue;
            }
            let Ok(value) = attribute.normalized_value(quick_xml::XmlVersion::Implicit1_0) else {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::XmlSyntax,
                        "attribute value contains an invalid entity reference; attribute ignored",
                    )
                    .with_location(self.position()),
                );
                continue;
            };
            let (resolve, local) = self.reader.resolver().resolve_attribute(attribute.key);
            match resolve {
                ResolveResult::Bound(namespace)
                    if namespace.0 == XSI && local.into_inner() == "type" =>
                {
                    xsi_type = Some(self.resolve_qname_value(&value));
                }
                ResolveResult::Bound(_) => {
                    // Namespaced extension attributes are tolerated and ignored.
                    self.ignored_elements += 1;
                }
                ResolveResult::Unknown(_) => {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            DiagnosticCode::XmlUnresolvedPrefix,
                            "attribute uses an undeclared prefix; attribute ignored",
                        )
                        .with_location(self.position()),
                    );
                }
                ResolveResult::Unbound => {
                    plain.push((local.into_inner().to_string(), value.into_owned()));
                }
            }
        }
        (plain, xsi_type)
    }

    /// Resolves a QName-valued attribute (e.g. `xsi:type="bpmn:serviceTask"`)
    /// to a local name; unqualified values pass through. A value whose prefix
    /// binds to a non-BPMN namespace resolves to the empty string.
    fn resolve_qname_value(&mut self, value: &str) -> String {
        let trimmed = value.trim();
        let Some((prefix, local)) = trimmed.split_once(':') else {
            return trimmed.to_string();
        };
        let probe = format!("{prefix}:probe");
        let (resolve, _) = self
            .reader
            .resolver()
            .resolve_attribute(QName(probe.as_str()));
        match resolve {
            ResolveResult::Bound(namespace) if namespace.0 == BPMN_MODEL => local.to_string(),
            ResolveResult::Bound(_) => String::new(),
            _ => local.to_string(),
        }
    }

    fn enter(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(ParseError {
                diagnostic: Diagnostic::error(
                    DiagnosticCode::XmlDepthLimit,
                    format!(
                        "nesting depth exceeds the configured limit of {}",
                        self.max_depth
                    ),
                )
                .with_location(self.position()),
            });
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn parse_document(&mut self) -> Result<(), ParseError> {
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(start) => {
                    self.enter()?;
                    let name = self.resolve_name(start.name());
                    if !name.matches(BPMN_MODEL, "definitions") {
                        return Err(ParseError {
                            diagnostic: Diagnostic::error(
                                DiagnosticCode::DefinitionsMissingRoot,
                                format!(
                                    "expected root element `definitions` in the BPMN namespace, \
                                     found `{}`",
                                    name.local
                                ),
                            )
                            .with_location(self.position()),
                        });
                    }
                    let (attrs, _) = self.attributes(&start);
                    self.definitions.id = attr_value(&attrs, "id");
                    self.definitions.target_namespace = attr_value(&attrs, "targetNamespace");
                    let result = self.parse_definitions_children();
                    self.leave();
                    return result;
                }
                Event::Empty(start) => {
                    let name = self.resolve_name(start.name());
                    if !name.matches(BPMN_MODEL, "definitions") {
                        return Err(ParseError {
                            diagnostic: Diagnostic::error(
                                DiagnosticCode::DefinitionsMissingRoot,
                                format!(
                                    "expected root element `definitions` in the BPMN namespace, \
                                     found `{}`",
                                    name.local
                                ),
                            )
                            .with_location(self.position()),
                        });
                    }
                    let (attrs, _) = self.attributes(&start);
                    self.definitions.id = attr_value(&attrs, "id");
                    self.definitions.target_namespace = attr_value(&attrs, "targetNamespace");
                    return Ok(());
                }
                Event::Eof => {
                    return Err(ParseError {
                        diagnostic: Diagnostic::error(
                            DiagnosticCode::DefinitionsMissingRoot,
                            "document contains no root element",
                        )
                        .with_location(self.position()),
                    });
                }
                _ => {}
            }
        }
    }

    fn parse_definitions_children(&mut self) -> Result<(), ParseError> {
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(start) => {
                    self.enter()?;
                    let name = self.resolve_name(start.name());
                    let result = if name.matches(BPMN_MODEL, "process") {
                        self.parse_process(&start).map(|process| {
                            self.definitions
                                .root_elements
                                .push(RootElement::Process(process))
                        })
                    } else if name.matches(BPMN_MODEL, "collaboration") {
                        self.parse_collaboration(&start).map(|collaboration| {
                            self.definitions
                                .root_elements
                                .push(RootElement::Collaboration(collaboration))
                        })
                    } else if name.matches(BPMN_DI, "BPMNDiagram") {
                        self.parse_diagram(&start, false)
                            .map(|diagram| self.definitions.diagrams.push(diagram))
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&start)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(start) => {
                    let name = self.resolve_name(start.name());
                    if name.matches(BPMN_DI, "BPMNDiagram") {
                        let diagram = self.parse_diagram(&start, true)?;
                        self.definitions.diagrams.push(diagram);
                    } else {
                        self.ignored_elements += 1;
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(());
                }
                Event::Eof => {
                    return Err(
                        self.syntax_error("unexpected end of document inside `definitions`")
                    );
                }
                _ => {}
            }
        }
    }

    /// Skips the subtree opened by `start` (whose Start event was consumed).
    fn skip_subtree(&mut self, start: &BytesStart<'_>) -> Result<(), ParseError> {
        let element_name = start.name().into_inner().to_string();
        let mut depth = 1usize;
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(_) => {
                    depth += 1;
                    if depth > self.max_depth {
                        return Err(ParseError {
                            diagnostic: Diagnostic::error(
                                DiagnosticCode::XmlDepthLimit,
                                format!(
                                    "nesting depth exceeds the configured limit of {}",
                                    self.max_depth
                                ),
                            )
                            .with_location(self.position()),
                        });
                    }
                }
                Event::Empty(_) => {}
                Event::End(_) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Event::Eof => {
                    return Err(self.syntax_error(format!(
                        "unexpected end of document inside `<{element_name}>`"
                    )));
                }
                _ => {}
            }
        }
    }

    fn parse_process(&mut self, start: &BytesStart<'_>) -> Result<Process, ParseError> {
        let (attrs, _) = self.attributes(start);
        let mut process = Process {
            id: attr_value(&attrs, "id"),
            name: attr_value(&attrs, "name"),
            is_executable: attr_bool(&attrs, "isExecutable"),
            ..Process::default()
        };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "laneSet") {
                        self.parse_lane_set(&child, false).map(|lane_set| {
                            process.lane_sets.push(lane_set);
                        })
                    } else if name.matches(BPMN_MODEL, "textAnnotation") {
                        self.parse_text_annotation(&child, false).map(|annotation| {
                            if let Some(annotation) = annotation {
                                process.artifacts.push(Artifact::TextAnnotation(annotation));
                            }
                        })
                    } else if name.matches(BPMN_MODEL, "ioSpecification") {
                        self.parse_io_specification(&child)
                    } else if name.namespace.as_deref() == Some(BPMN_MODEL)
                        && (Parser::is_data_shape(&name.local) || is_association_name(&name.local))
                    {
                        let (child_attrs, _) = self.attributes(&child);
                        self.note_unsupported_element(&child_attrs, &name.local);
                        self.skip_subtree(&child)
                    } else if is_flow_element_name(&name) {
                        self.parse_flow_element(&child, &name, false)
                            .map(|element| {
                                if let Some(element) = element {
                                    process.flow_elements.push(element);
                                }
                            })
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(BPMN_MODEL, "laneSet") {
                        let lane_set = self.parse_lane_set(&child, true)?;
                        process.lane_sets.push(lane_set);
                    } else if name.matches(BPMN_MODEL, "textAnnotation") {
                        if let Some(annotation) = self.parse_text_annotation(&child, true)? {
                            process.artifacts.push(Artifact::TextAnnotation(annotation));
                        }
                    } else if name.namespace.as_deref() == Some(BPMN_MODEL)
                        && (Parser::is_data_shape(&name.local) || is_association_name(&name.local))
                    {
                        let (child_attrs, _) = self.attributes(&child);
                        self.note_unsupported_element(&child_attrs, &name.local);
                    } else if is_flow_element_name(&name) {
                        if let Some(element) = self.parse_flow_element(&child, &name, true)? {
                            process.flow_elements.push(element);
                        }
                    } else {
                        self.ignored_elements += 1;
                    }
                }
                Event::End(_) => {
                    self.leave();
                    apply_default_flags(&mut process.flow_elements);
                    return Ok(process);
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `process`"));
                }
                _ => {}
            }
        }
    }

    /// Records a data-shape or association element for id/DI resolution.
    fn note_unsupported_element(&mut self, attrs: &[(String, String)], local: &str) {
        if let Some(id) = attr_value(attrs, "id").filter(|id| !id.is_empty()) {
            self.unsupported.push(UnsupportedElement {
                id,
                local_name: local.to_string(),
            });
        }
    }

    /// True when `local` is a data shape that needs a DI shape to render.
    fn is_data_shape(local: &str) -> bool {
        matches!(
            local,
            "dataObject"
                | "dataObjectReference"
                | "dataStoreReference"
                | "dataInput"
                | "dataOutput"
        )
    }

    /// `ioSpecification` children (`dataInput`/`dataOutput`) are recorded as
    /// unsupported elements; everything else inside is skipped.
    fn parse_io_specification(&mut self, _start: &BytesStart<'_>) -> Result<(), ParseError> {
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if matches!(name.local.as_str(), "dataInput" | "dataOutput")
                        && name.namespace.as_deref() == Some(BPMN_MODEL)
                    {
                        let (child_attrs, _) = self.attributes(&child);
                        self.note_unsupported_element(&child_attrs, &name.local);
                        self.skip_subtree(&child)
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if matches!(name.local.as_str(), "dataInput" | "dataOutput")
                        && name.namespace.as_deref() == Some(BPMN_MODEL)
                    {
                        let (child_attrs, _) = self.attributes(&child);
                        self.note_unsupported_element(&child_attrs, &name.local);
                    }
                }
                Event::End(_) => return Ok(()),
                Event::Eof => {
                    return Err(
                        self.syntax_error("unexpected end of document inside `ioSpecification`")
                    );
                }
                _ => {}
            }
        }
    }

    fn parse_lane_set(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<LaneSet, ParseError> {
        let (attrs, _) = self.attributes(start);
        if empty {
            return Ok(LaneSet {
                id: attr_value(&attrs, "id"),
                name: attr_value(&attrs, "name"),
                lanes: Vec::new(),
            });
        }
        let mut lane_set = LaneSet {
            id: attr_value(&attrs, "id"),
            name: attr_value(&attrs, "name"),
            lanes: Vec::new(),
        };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "lane") {
                        self.parse_lane(&child, false)
                            .map(|lane| lane_set.lanes.push(lane))
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(BPMN_MODEL, "lane") {
                        let lane = self.parse_lane(&child, true)?;
                        lane_set.lanes.push(lane);
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(lane_set);
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `laneSet`"));
                }
                _ => {}
            }
        }
    }

    fn parse_lane(&mut self, start: &BytesStart<'_>, empty: bool) -> Result<Lane, ParseError> {
        let (attrs, _) = self.attributes(start);
        if empty {
            return Ok(Lane {
                id: attr_value(&attrs, "id").unwrap_or_default(),
                name: attr_value(&attrs, "name"),
                flow_node_refs: Vec::new(),
            });
        }
        let mut lane = Lane {
            id: attr_value(&attrs, "id").unwrap_or_default(),
            name: attr_value(&attrs, "name"),
            flow_node_refs: Vec::new(),
        };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "flowNodeRef") {
                        self.capture_text(&child).map(|text| {
                            if let Some(text) = text {
                                lane.flow_node_refs.push(text.trim().to_string());
                            }
                        })
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(_) => {}
                Event::End(_) => {
                    self.leave();
                    return Ok(lane);
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `lane`"));
                }
                _ => {}
            }
        }
    }

    fn parse_text_annotation(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<TextAnnotation>, ParseError> {
        let (attrs, _) = self.attributes(start);
        if empty {
            return Ok(Some(TextAnnotation {
                id: attr_value(&attrs, "id").unwrap_or_default(),
                text: None,
            }));
        }
        let Some(id) = attr_value(&attrs, "id") else {
            self.diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::IgnoredElement,
                    "textAnnotation without id cannot be referenced; ignored",
                )
                .with_location(self.position()),
            );
            self.skip_subtree(start)?;
            return Ok(None);
        };
        let mut annotation = TextAnnotation { id, text: None };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "text") {
                        self.capture_text(&child).map(|text| annotation.text = text)
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(_) => {}
                Event::End(_) => {
                    self.leave();
                    return Ok(Some(annotation));
                }
                Event::Eof => {
                    return Err(
                        self.syntax_error("unexpected end of document inside `textAnnotation`")
                    );
                }
                _ => {}
            }
        }
    }

    /// Captures the text content of an element opened by `start`.
    fn capture_text(&mut self, _start: &BytesStart<'_>) -> Result<Option<String>, ParseError> {
        let mut text = String::new();
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Text(t) => match quick_xml::escape::unescape(t.into_inner().as_ref()) {
                    Ok(decoded) => text.push_str(&decoded),
                    Err(_) => {
                        self.diagnostics.push(
                            Diagnostic::warning(
                                DiagnosticCode::XmlSyntax,
                                "text contains an invalid entity reference; text truncated",
                            )
                            .with_location(self.position()),
                        );
                    }
                },
                Event::CData(t) => text.push_str(t.xml10_content().as_ref()),
                Event::Start(nested) => {
                    self.enter()?;
                    self.ignored_elements += 1;
                    let result = self.skip_subtree(&nested);
                    self.leave();
                    result?;
                }
                Event::Empty(_) => {}
                Event::End(_) => return Ok(Some(text)),
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside text element"));
                }
                _ => {}
            }
        }
    }

    fn parse_flow_element(
        &mut self,
        start: &BytesStart<'_>,
        name: &ResolvedName,
        empty: bool,
    ) -> Result<Option<FlowElement>, ParseError> {
        let (attrs, xsi_type) = self.attributes(start);
        self.parse_flow_element_with(start, name, empty, attrs, xsi_type)
    }

    /// [`Self::parse_flow_element`] with attributes already resolved; the
    /// subprocess loop dispatches here so an element is never scanned twice.
    fn parse_flow_element_with(
        &mut self,
        start: &BytesStart<'_>,
        name: &ResolvedName,
        empty: bool,
        attrs: Vec<(String, String)>,
        xsi_type: Option<String>,
    ) -> Result<Option<FlowElement>, ParseError> {
        if name.matches(BPMN_MODEL, "sequenceFlow") {
            return self.parse_sequence_flow(&attrs, start, empty);
        }
        let Some(id) = attr_value(&attrs, "id") else {
            self.diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::IgnoredElement,
                    format!(
                        "`{}` without id cannot be referenced or rendered; ignored",
                        name.local
                    ),
                )
                .with_location(self.position()),
            );
            if !empty {
                self.skip_subtree(start)?;
            }
            return Ok(None);
        };
        let node_name = name.local.clone();
        let default_flow_ref = attr_value(&attrs, "default");
        let empty_kind = |node_name: &str| -> NodeKind {
            if node_name == "boundaryEvent" {
                // Self-closing boundary events must not degrade to
                // `NodeKind::Other`; regression-tested in this module.
                NodeKind::BoundaryEvent {
                    attached_to: attr_value(&attrs, "attachedToRef"),
                    cancel_activity: attr_value(&attrs, "cancelActivity")
                        .map(|value| value != "false")
                        .unwrap_or(true),
                    definition: EventDefinition::None,
                }
            } else {
                kind_from_name(node_name, xsi_type.as_deref())
            }
        };
        let (content, kind) = if empty {
            (
                NodeContent {
                    default_flow_ref: default_flow_ref.clone(),
                    ..NodeContent::default()
                },
                empty_kind(&node_name),
            )
        } else if node_name == "subProcess" {
            let (content, body) = self.parse_subprocess_node(start)?;
            return Ok(Some(finished_subprocess(&attrs, content, body)));
        } else {
            let content = self.parse_node_children(start)?;
            let kind = match node_name.as_str() {
                "startEvent" => NodeKind::StartEvent {
                    definition: content.event_definition,
                },
                "endEvent" => NodeKind::EndEvent {
                    definition: content.event_definition,
                },
                "intermediateCatchEvent" => NodeKind::IntermediateCatchEvent {
                    definition: content.event_definition,
                },
                "intermediateThrowEvent" => NodeKind::IntermediateThrowEvent {
                    definition: content.event_definition,
                },
                "boundaryEvent" => NodeKind::BoundaryEvent {
                    attached_to: attr_value(&attrs, "attachedToRef"),
                    cancel_activity: attr_value(&attrs, "cancelActivity")
                        .map(|value| value != "false")
                        .unwrap_or(true),
                    definition: content.event_definition,
                },
                _ => kind_from_name(&node_name, xsi_type.as_deref()),
            };
            (content, kind)
        };
        let mut content = content;
        content.default_flow_ref = default_flow_ref;
        Ok(Some(FlowElement::Node(FlowNode {
            id,
            name: attr_value(&attrs, "name"),
            incoming: content.incoming,
            outgoing: content.outgoing,
            default_flow_ref: content.default_flow_ref,
            kind,
        })))
    }

    fn parse_sequence_flow(
        &mut self,
        attrs: &[(String, String)],
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<FlowElement>, ParseError> {
        let Some(id) = attr_value(attrs, "id") else {
            self.diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::IgnoredElement,
                    "sequenceFlow without id cannot be referenced; ignored",
                )
                .with_location(self.position()),
            );
            if !empty {
                self.skip_subtree(start)?;
            }
            return Ok(None);
        };
        let source_ref = attr_value(attrs, "sourceRef").unwrap_or_default();
        let target_ref = attr_value(attrs, "targetRef").unwrap_or_default();
        if source_ref.is_empty() || target_ref.is_empty() {
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DanglingReference,
                    "sequenceFlow is missing sourceRef or targetRef",
                )
                .with_element_id(id.clone())
                .with_location(self.position()),
            );
        }
        if empty {
            return Ok(Some(FlowElement::Flow(SequenceFlow {
                id,
                name: attr_value(attrs, "name"),
                source_ref,
                target_ref,
                is_default: false,
                condition: None,
            })));
        }
        let mut condition = None;
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "conditionExpression") {
                        self.capture_text(&child).map(|text| condition = text)
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(_) => {}
                Event::End(_) => {
                    self.leave();
                    return Ok(Some(FlowElement::Flow(SequenceFlow {
                        id,
                        name: attr_value(attrs, "name"),
                        source_ref,
                        target_ref,
                        is_default: false,
                        condition,
                    })));
                }
                Event::Eof => {
                    return Err(
                        self.syntax_error("unexpected end of document inside `sequenceFlow`")
                    );
                }
                _ => {}
            }
        }
    }

    /// Consumes children of a flow node: `incoming`, `outgoing`, event
    /// definition markers, and everything else (skipped).
    fn parse_node_children(&mut self, _start: &BytesStart<'_>) -> Result<NodeContent, ParseError> {
        let mut content = NodeContent::default();
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "incoming") {
                        self.capture_text(&child).map(|text| {
                            if let Some(text) = text {
                                content.incoming.push(text.trim().to_string());
                            }
                        })
                    } else if name.matches(BPMN_MODEL, "outgoing") {
                        self.capture_text(&child).map(|text| {
                            if let Some(text) = text {
                                content.outgoing.push(text.trim().to_string());
                            }
                        })
                    } else if name.matches(BPMN_MODEL, "ioSpecification") {
                        self.parse_io_specification(&child)
                    } else if name.namespace.as_deref() == Some(BPMN_MODEL)
                        && (Parser::is_data_shape(&name.local) || is_association_name(&name.local))
                    {
                        let (child_attrs, _) = self.attributes(&child);
                        self.note_unsupported_element(&child_attrs, &name.local);
                        self.skip_subtree(&child)
                    } else {
                        if let Some(found) =
                            event_definition(&name.local, name.namespace.as_deref())
                        {
                            content.event_definition = found;
                        }
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if let Some(found) = event_definition(&name.local, name.namespace.as_deref()) {
                        content.event_definition = found;
                    }
                }
                Event::End(_) => return Ok(content),
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside flow node"));
                }
                _ => {}
            }
        }
    }

    /// Subprocess children: nested flow elements plus node references.
    ///
    /// `subProcess` nesting is tracked on an explicit heap stack instead of
    /// native recursion (SPEC-001 section 4): one set of parsing frames
    /// serves every level, so the configured depth limit can never exhaust
    /// the thread stack.
    fn parse_subprocess_node(
        &mut self,
        _start: &BytesStart<'_>,
    ) -> Result<(NodeContent, Vec<FlowElement>), ParseError> {
        /// One open `subProcess` element and its accumulated body.
        struct SubprocessFrame {
            attrs: Vec<(String, String)>,
            content: NodeContent,
            body: Vec<FlowElement>,
        }

        let mut stack = vec![SubprocessFrame {
            attrs: Vec::new(),
            content: NodeContent::default(),
            body: Vec::new(),
        }];
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let is_flow = is_flow_element_name(&name);
                    if is_flow {
                        let (attrs, xsi_type) = self.attributes(&child);
                        if name.local == "subProcess" && attr_value(&attrs, "id").is_some() {
                            // Nested non-empty subprocess: open a heap frame
                            // instead of recursing; depth stays elevated until
                            // its End event pops the frame below.
                            stack.push(SubprocessFrame {
                                attrs,
                                content: NodeContent::default(),
                                body: Vec::new(),
                            });
                            continue;
                        }
                        let parsed =
                            self.parse_flow_element_with(&child, &name, false, attrs, xsi_type);
                        if let (Some(element), Some(frame)) = (parsed?, stack.last_mut()) {
                            frame.body.push(element);
                        }
                    } else if name.matches(BPMN_MODEL, "incoming") {
                        let text = self.capture_text(&child)?;
                        if let (Some(text), Some(frame)) = (text, stack.last_mut()) {
                            frame.content.incoming.push(text.trim().to_string());
                        }
                    } else if name.matches(BPMN_MODEL, "outgoing") {
                        let text = self.capture_text(&child)?;
                        if let (Some(text), Some(frame)) = (text, stack.last_mut()) {
                            frame.content.outgoing.push(text.trim().to_string());
                        }
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)?;
                    }
                    self.leave();
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if is_flow_element_name(&name)
                        && let Some(element) = self.parse_flow_element(&child, &name, true)?
                        && let Some(frame) = stack.last_mut()
                    {
                        frame.body.push(element);
                    }
                }
                Event::End(_) => {
                    self.leave();
                    let frame = stack
                        .pop()
                        .expect("subprocess stack always holds the bottom frame");
                    let Some(parent) = stack.last_mut() else {
                        return Ok((frame.content, frame.body));
                    };
                    parent
                        .body
                        .push(finished_subprocess(&frame.attrs, frame.content, frame.body));
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `subProcess`"));
                }
                _ => {}
            }
        }
    }

    fn parse_collaboration(&mut self, start: &BytesStart<'_>) -> Result<Collaboration, ParseError> {
        let (attrs, _) = self.attributes(start);
        let mut collaboration = Collaboration {
            id: attr_value(&attrs, "id"),
            name: attr_value(&attrs, "name"),
            participants: Vec::new(),
            message_flows: Vec::new(),
        };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_MODEL, "participant") {
                        self.parse_participant(&child, false).map(|participant| {
                            if let Some(participant) = participant {
                                collaboration.participants.push(participant);
                            }
                        })
                    } else if name.matches(BPMN_MODEL, "messageFlow") {
                        self.parse_message_flow(&child, false).map(|flow| {
                            if let Some(flow) = flow {
                                collaboration.message_flows.push(flow);
                            }
                        })
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(BPMN_MODEL, "participant") {
                        if let Some(participant) = self.parse_participant(&child, true)? {
                            collaboration.participants.push(participant);
                        }
                    } else if name.matches(BPMN_MODEL, "messageFlow")
                        && let Some(flow) = self.parse_message_flow(&child, true)?
                    {
                        collaboration.message_flows.push(flow);
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(collaboration);
                }
                Event::Eof => {
                    return Err(
                        self.syntax_error("unexpected end of document inside `collaboration`")
                    );
                }
                _ => {}
            }
        }
    }

    fn parse_participant(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<Participant>, ParseError> {
        let (attrs, _) = self.attributes(start);
        let Some(id) = attr_value(&attrs, "id") else {
            self.diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::IgnoredElement,
                    "participant without id ignored",
                )
                .with_location(self.position()),
            );
            if !empty {
                self.skip_subtree(start)?;
            }
            return Ok(None);
        };
        if !empty {
            self.skip_subtree(start)?;
        }
        Ok(Some(Participant {
            id,
            name: attr_value(&attrs, "name"),
            process_ref: attr_value(&attrs, "processRef"),
        }))
    }

    fn parse_message_flow(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<MessageFlow>, ParseError> {
        let (attrs, _) = self.attributes(start);
        let Some(id) = attr_value(&attrs, "id") else {
            self.diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::IgnoredElement,
                    "messageFlow without id ignored",
                )
                .with_location(self.position()),
            );
            if !empty {
                self.skip_subtree(start)?;
            }
            return Ok(None);
        };
        if !empty {
            self.skip_subtree(start)?;
        }
        Ok(Some(MessageFlow {
            id,
            name: attr_value(&attrs, "name"),
            source_ref: attr_value(&attrs, "sourceRef").unwrap_or_default(),
            target_ref: attr_value(&attrs, "targetRef").unwrap_or_default(),
        }))
    }

    fn parse_diagram(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Diagram, ParseError> {
        let (attrs, _) = self.attributes(start);
        if empty {
            return Ok(Diagram {
                id: attr_value(&attrs, "id"),
                name: attr_value(&attrs, "name"),
                plane: Plane::default(),
            });
        }
        let mut diagram = Diagram {
            id: attr_value(&attrs, "id"),
            name: attr_value(&attrs, "name"),
            plane: Plane::default(),
        };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_DI, "BPMNPlane") {
                        self.parse_plane(&child, false)
                            .map(|plane| diagram.plane = plane)
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(BPMN_DI, "BPMNPlane") {
                        let plane = self.parse_plane(&child, true)?;
                        diagram.plane = plane;
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(diagram);
                }
                Event::Eof => {
                    return Err(
                        self.syntax_error("unexpected end of document inside `BPMNDiagram`")
                    );
                }
                _ => {}
            }
        }
    }

    fn parse_plane(&mut self, start: &BytesStart<'_>, empty: bool) -> Result<Plane, ParseError> {
        let (attrs, _) = self.attributes(start);
        if empty {
            return Ok(Plane {
                id: attr_value(&attrs, "id"),
                bpmn_element: attr_value(&attrs, "bpmnElement"),
                shapes: Vec::new(),
                edges: Vec::new(),
            });
        }
        let mut plane = Plane {
            id: attr_value(&attrs, "id"),
            bpmn_element: attr_value(&attrs, "bpmnElement"),
            shapes: Vec::new(),
            edges: Vec::new(),
        };
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(BPMN_DI, "BPMNShape") {
                        self.parse_shape(&child, false).map(|shape| {
                            if let Some(shape) = shape {
                                plane.shapes.push(shape);
                            }
                        })
                    } else if name.matches(BPMN_DI, "BPMNEdge") {
                        self.parse_edge(&child, false).map(|edge| {
                            if let Some(edge) = edge {
                                plane.edges.push(edge);
                            }
                        })
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(BPMN_DI, "BPMNShape") {
                        if let Some(shape) = self.parse_shape(&child, true)? {
                            plane.shapes.push(shape);
                        }
                    } else if name.matches(BPMN_DI, "BPMNEdge")
                        && let Some(edge) = self.parse_edge(&child, true)?
                    {
                        plane.edges.push(edge);
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(plane);
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `BPMNPlane`"));
                }
                _ => {}
            }
        }
    }

    fn parse_shape(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<Shape>, ParseError> {
        let (attrs, _) = self.attributes(start);
        if empty {
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidGeometry,
                    "self-closing BPMNShape cannot carry dc:Bounds; shape ignored",
                )
                .with_location(self.position()),
            );
            return Ok(None);
        }
        let Some(bpmn_element) = attr_value(&attrs, "bpmnElement") else {
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DanglingDiReference,
                    "BPMNShape is missing the required bpmnElement attribute; shape ignored",
                )
                .with_location(self.position()),
            );
            self.skip_subtree(start)?;
            return Ok(None);
        };
        let shape_id = attr_value(&attrs, "id");
        let mut bounds: Option<Bounds> = None;
        let mut label: Option<BpmnLabel> = None;
        let mut bounds_error = false;
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(DC, "Bounds") {
                        self.capture_shape_bounds(&child, &mut bounds, &mut bounds_error)
                    } else if name.matches(BPMN_DI, "BPMNLabel") {
                        self.parse_label(&child).map(|found| label = Some(found))
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(DC, "Bounds") {
                        let (bounds_attrs, _) = self.attributes(&child);
                        match parse_bounds(&bounds_attrs) {
                            Some(found) => bounds = Some(found),
                            None => bounds_error = true,
                        }
                    } else if name.matches(BPMN_DI, "BPMNLabel") {
                        label = Some(BpmnLabel::default());
                    }
                }
                Event::End(_) => {
                    self.leave();
                    if bounds_error {
                        self.diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidGeometry,
                                "BPMNShape has unparseable dc:Bounds; shape ignored",
                            )
                            .with_element_id(bpmn_element.clone())
                            .with_location(self.position()),
                        );
                        return Ok(None);
                    }
                    let Some(bounds) = bounds else {
                        self.diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidGeometry,
                                "BPMNShape is missing dc:Bounds; shape ignored",
                            )
                            .with_element_id(bpmn_element.clone())
                            .with_location(self.position()),
                        );
                        return Ok(None);
                    };
                    return Ok(Some(Shape {
                        id: shape_id,
                        bpmn_element,
                        bounds,
                        label,
                        is_horizontal: attr_bool(&attrs, "isHorizontal"),
                        is_expanded: attr_bool(&attrs, "isExpanded"),
                    }));
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `BPMNShape`"));
                }
                _ => {}
            }
        }
    }

    /// Reads a `dc:Bounds` child of a shape opened by `child`.
    fn capture_shape_bounds(
        &mut self,
        child: &BytesStart<'_>,
        bounds: &mut Option<Bounds>,
        bounds_error: &mut bool,
    ) -> Result<(), ParseError> {
        // The Bounds element is expected to be empty; consume until End.
        let (attrs, _) = self.attributes(child);
        match parse_bounds(&attrs) {
            Some(found) => *bounds = Some(found),
            None => *bounds_error = true,
        }
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(nested) => {
                    self.enter()?;
                    let result = self.skip_subtree(&nested);
                    self.leave();
                    result?;
                }
                Event::Empty(_) => {}
                Event::End(_) => return Ok(()),
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `dc:Bounds`"));
                }
                _ => {}
            }
        }
    }

    fn parse_edge(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<Edge>, ParseError> {
        let (attrs, _) = self.attributes(start);
        let Some(bpmn_element) = attr_value(&attrs, "bpmnElement") else {
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DanglingDiReference,
                    "BPMNEdge is missing the required bpmnElement attribute; edge ignored",
                )
                .with_location(self.position()),
            );
            if !empty {
                self.skip_subtree(start)?;
            }
            return Ok(None);
        };
        if empty {
            return Ok(Some(Edge {
                id: attr_value(&attrs, "id"),
                bpmn_element,
                waypoints: Vec::new(),
                label: None,
            }));
        };
        let edge_id = attr_value(&attrs, "id");
        let mut waypoints = Vec::new();
        let mut label: Option<BpmnLabel> = None;
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(DI, "waypoint") {
                        self.parse_waypoint(&child, false).map(|point| {
                            if let Some(point) = point {
                                waypoints.push(point);
                            }
                        })
                    } else if name.matches(BPMN_DI, "BPMNLabel") {
                        self.parse_label(&child).map(|found| label = Some(found))
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(DI, "waypoint") {
                        if let Some(point) = self.parse_waypoint(&child, true)? {
                            waypoints.push(point);
                        }
                    } else if name.matches(BPMN_DI, "BPMNLabel") {
                        label = Some(BpmnLabel::default());
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(Some(Edge {
                        id: edge_id,
                        bpmn_element,
                        waypoints,
                        label,
                    }));
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `BPMNEdge`"));
                }
                _ => {}
            }
        }
    }

    fn parse_waypoint(
        &mut self,
        child: &BytesStart<'_>,
        empty: bool,
    ) -> Result<Option<Point>, ParseError> {
        let (attrs, _) = self.attributes(child);
        let parsed = parse_point(&attrs);
        if !empty {
            self.skip_subtree(child)?;
        }
        if parsed.is_none() {
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidGeometry,
                    "di:waypoint is missing or has unparseable x/y; waypoint dropped",
                )
                .with_location(self.position()),
            );
        }
        Ok(parsed)
    }

    fn parse_label(&mut self, _start: &BytesStart<'_>) -> Result<BpmnLabel, ParseError> {
        let mut bounds = None;
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(child) => {
                    self.enter()?;
                    let name = self.resolve_name(child.name());
                    let result = if name.matches(DC, "Bounds") {
                        self.capture_label_bounds(&child, &mut bounds)
                    } else {
                        self.ignored_elements += 1;
                        self.skip_subtree(&child)
                    };
                    self.leave();
                    result?;
                }
                Event::Empty(child) => {
                    let name = self.resolve_name(child.name());
                    if name.matches(DC, "Bounds") {
                        let (bounds_attrs, _) = self.attributes(&child);
                        bounds = parse_bounds(&bounds_attrs);
                    }
                }
                Event::End(_) => {
                    self.leave();
                    return Ok(BpmnLabel { bounds });
                }
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `BPMNLabel`"));
                }
                _ => {}
            }
        }
    }

    fn capture_label_bounds(
        &mut self,
        child: &BytesStart<'_>,
        bounds: &mut Option<Bounds>,
    ) -> Result<(), ParseError> {
        let (attrs, _) = self.attributes(child);
        *bounds = parse_bounds(&attrs);
        loop {
            let (_, event) = match self.reader.read_resolved_event() {
                Ok(pair) => pair,
                Err(error) => return Err(self.syntax_error(format!("malformed XML: {error}"))),
            };
            match event {
                Event::Start(nested) => {
                    self.enter()?;
                    let result = self.skip_subtree(&nested);
                    self.leave();
                    result?;
                }
                Event::Empty(_) => {}
                Event::End(_) => return Ok(()),
                Event::Eof => {
                    return Err(self.syntax_error("unexpected end of document inside `dc:Bounds`"));
                }
                _ => {}
            }
        }
    }
}

fn attr_value(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

fn attr_bool(attrs: &[(String, String)], key: &str) -> Option<bool> {
    attr_value(attrs, key).map(|value| value == "true" || value == "1")
}

fn parse_f64(attrs: &[(String, String)], key: &str) -> Option<f64> {
    let raw = attr_value(attrs, key)?;
    raw.trim().parse::<f64>().ok()
}

fn parse_bounds(attrs: &[(String, String)]) -> Option<Bounds> {
    let x = parse_f64(attrs, "x")?;
    let y = parse_f64(attrs, "y")?;
    let width = parse_f64(attrs, "width")?;
    let height = parse_f64(attrs, "height")?;
    Some(Bounds::new(x, y, width, height))
}

fn parse_point(attrs: &[(String, String)]) -> Option<Point> {
    let x = parse_f64(attrs, "x")?;
    let y = parse_f64(attrs, "y")?;
    Some(Point::new(x, y))
}

fn event_definition(local: &str, namespace: Option<&str>) -> Option<EventDefinition> {
    if namespace != Some(BPMN_MODEL) {
        return None;
    }
    let definition = match local {
        "messageEventDefinition" => EventDefinition::Message,
        "timerEventDefinition" => EventDefinition::Timer,
        "errorEventDefinition" => EventDefinition::Error,
        "escalationEventDefinition" => EventDefinition::Escalation,
        "terminateEventDefinition" => EventDefinition::Terminate,
        "compensateEventDefinition" => EventDefinition::Compensation,
        "conditionalEventDefinition" => EventDefinition::Conditional,
        "linkEventDefinition" => EventDefinition::Link,
        "signalEventDefinition" => EventDefinition::Signal,
        "cancelEventDefinition" => EventDefinition::Cancel,
        "multipleEventDefinition" | "parallelMultipleEventDefinition" => EventDefinition::Multiple,
        _ => return None,
    };
    Some(definition)
}

fn task_type(local: &str, xsi_type: Option<&str>) -> Option<TaskType> {
    let effective = xsi_type.filter(|value| !value.is_empty()).unwrap_or(local);
    match effective {
        "task" => Some(TaskType::Abstract),
        "userTask" => Some(TaskType::User),
        "serviceTask" => Some(TaskType::Service),
        "manualTask" => Some(TaskType::Manual),
        "scriptTask" => Some(TaskType::Script),
        "sendTask" => Some(TaskType::Send),
        "receiveTask" => Some(TaskType::Receive),
        "businessRuleTask" => Some(TaskType::BusinessRule),
        _ => None,
    }
}

fn gateway_type(local: &str, xsi_type: Option<&str>) -> Option<GatewayType> {
    let effective = xsi_type.filter(|value| !value.is_empty()).unwrap_or(local);
    match effective {
        "exclusiveGateway" => Some(GatewayType::Exclusive),
        "parallelGateway" => Some(GatewayType::Parallel),
        "inclusiveGateway" => Some(GatewayType::Inclusive),
        "eventBasedGateway" => Some(GatewayType::EventBased),
        "complexGateway" => Some(GatewayType::Complex),
        _ => None,
    }
}

/// Marks sequence flows that are the `default` of a source node.
fn apply_default_flags(body: &mut [FlowElement]) {
    let defaults: Vec<String> = body
        .iter()
        .filter_map(|element| match element {
            FlowElement::Node(node) => node.default_flow_ref.clone(),
            FlowElement::Flow(_) => None,
        })
        .collect();
    if defaults.is_empty() {
        return;
    }
    for element in body {
        if let FlowElement::Flow(flow) = element
            && defaults.contains(&flow.id)
        {
            flow.is_default = true;
        }
    }
}

/// Assembles the `FlowElement` for a completed non-empty `subProcess`.
/// Shared by the direct parse path and the frame pop in
/// `parse_subprocess_node` so both produce identical nodes.
fn finished_subprocess(
    attrs: &[(String, String)],
    mut content: NodeContent,
    mut body: Vec<FlowElement>,
) -> FlowElement {
    content.default_flow_ref = attr_value(attrs, "default");
    apply_default_flags(&mut body);
    FlowElement::Node(FlowNode {
        id: attr_value(attrs, "id").unwrap_or_default(),
        name: attr_value(attrs, "name"),
        incoming: content.incoming,
        outgoing: content.outgoing,
        default_flow_ref: content.default_flow_ref,
        kind: NodeKind::SubProcess {
            triggered_by_event: attr_value(attrs, "triggeredByEvent").is_some_and(|v| v == "true"),
            body,
        },
    })
}

/// Symbol kind for a node name without reading children (empty elements).
fn kind_from_name(node_name: &str, xsi_type: Option<&str>) -> NodeKind {
    match node_name {
        "startEvent" => NodeKind::StartEvent {
            definition: EventDefinition::None,
        },
        "endEvent" => NodeKind::EndEvent {
            definition: EventDefinition::None,
        },
        "intermediateCatchEvent" => NodeKind::IntermediateCatchEvent {
            definition: EventDefinition::None,
        },
        "intermediateThrowEvent" => NodeKind::IntermediateThrowEvent {
            definition: EventDefinition::None,
        },
        _ => {
            if let Some(gateway) = gateway_type(node_name, xsi_type) {
                NodeKind::Gateway {
                    gateway_type: gateway,
                }
            } else if let Some(task_type) = task_type(node_name, xsi_type) {
                NodeKind::Task { task_type }
            } else {
                NodeKind::Other {
                    local_name: node_name.to_string(),
                }
            }
        }
    }
}

/// True for association-flavored elements (P1 rendering, id resolution only).
fn is_association_name(local: &str) -> bool {
    matches!(
        local,
        "association" | "dataInputAssociation" | "dataOutputAssociation"
    )
}

/// True for element names the parser models as flow elements.
fn is_flow_element_name(name: &ResolvedName) -> bool {
    if name.namespace.as_deref() != Some(BPMN_MODEL) {
        return false;
    }
    matches!(
        name.local.as_str(),
        "startEvent"
            | "endEvent"
            | "intermediateCatchEvent"
            | "intermediateThrowEvent"
            | "boundaryEvent"
            | "task"
            | "userTask"
            | "serviceTask"
            | "manualTask"
            | "scriptTask"
            | "sendTask"
            | "receiveTask"
            | "businessRuleTask"
            | "gateway"
            | "exclusiveGateway"
            | "parallelGateway"
            | "inclusiveGateway"
            | "eventBasedGateway"
            | "complexGateway"
            | "subProcess"
            | "callActivity"
            | "transaction"
            | "sequenceFlow"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpmn_model::element::NodeKind;

    const MINIMAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"
                  xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                  xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
                  xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
                  id="Defs_1" targetNamespace="http://example.test/bpmn">
  <bpmn:process id="P1" isExecutable="false">
    <bpmn:startEvent id="Start_1" name="go">
      <bpmn:outgoing>Flow_1</bpmn:outgoing>
    </bpmn:startEvent>
    <bpmn:task id="Task_1" name="do &amp; done">
      <bpmn:incoming>Flow_1</bpmn:incoming>
      <bpmn:outgoing>Flow_2</bpmn:outgoing>
    </bpmn:task>
    <bpmn:endEvent id="End_1">
      <bpmn:incoming>Flow_2</bpmn:incoming>
    </bpmn:endEvent>
    <bpmn:sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_1"/>
    <bpmn:sequenceFlow id="Flow_2" sourceRef="Task_1" targetRef="End_1"/>
  </bpmn:process>
  <bpmndi:BPMNDiagram id="Diagram_1">
    <bpmndi:BPMNPlane id="Plane_1" bpmnElement="P1">
      <bpmndi:BPMNShape id="Start_1_di" bpmnElement="Start_1">
        <dc:Bounds x="160.0" y="80" width="36" height="36"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_1_di" bpmnElement="Task_1">
        <dc:Bounds x="250" y="60" width="100" height="80"/>
        <bpmndi:BPMNLabel><dc:Bounds x="250" y="90" width="100" height="20"/></bpmndi:BPMNLabel>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="End_1_di" bpmnElement="End_1">
        <dc:Bounds x="400" y="80" width="36" height="36"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Flow_1_di" bpmnElement="Flow_1">
        <di:waypoint x="196" y="98"/>
        <di:waypoint x="250" y="98"/>
      </bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_2_di" bpmnElement="Flow_2">
        <di:waypoint x="350" y="100"/>
        <di:waypoint x="400" y="100"/>
      </bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>
"#;

    #[test]
    fn parses_self_closing_boundary_event_as_boundary_event() {
        // Regression: a self-closing `<boundaryEvent/>` used to degrade to
        // `NodeKind::Other`, so attached boundary events lost their kind (and
        // layout warnings mislabeled them as unsupported elements).
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P">
    <task id="Host"/>
    <boundaryEvent id="Err" attachedToRef="Host"/>
  </process>
</definitions>
"#,
        )
        .expect("parse");
        let node = document.find_node("Err").expect("boundary event present");
        match &node.kind {
            NodeKind::BoundaryEvent {
                attached_to,
                cancel_activity,
                definition,
            } => {
                assert_eq!(attached_to.as_deref(), Some("Host"));
                assert!(*cancel_activity);
                assert_eq!(*definition, EventDefinition::None);
            }
            other => panic!("expected BoundaryEvent, got {other:?}"),
        }
    }

    #[test]
    fn parses_minimal_document_with_prefixed_names() {
        let document = from_xml(MINIMAL.as_bytes()).expect("parse must succeed");
        assert_eq!(document.definitions.id.as_deref(), Some("Defs_1"));
        assert_eq!(document.diagram_count(), 1);
        let process = document.processes().next().expect("one process");
        assert_eq!(process.flow_elements.len(), 5);
        let task = document.find_node("Task_1").expect("task present");
        assert_eq!(task.name.as_deref(), Some("do & done"));
        assert_eq!(task.incoming, vec!["Flow_1".to_string()]);
        let shape = &document.definitions.diagrams[0].plane.shapes[1];
        assert_eq!(shape.bpmn_element, "Task_1");
        assert_eq!(shape.bounds, Bounds::new(250.0, 60.0, 100.0, 80.0));
        assert!(shape.label.as_ref().and_then(|l| l.bounds).is_some());
    }

    #[test]
    fn parses_with_arbitrary_prefixes_and_default_ns() {
        let document = from_xml(
            br#"<?xml version="1.0"?>
<zzz:definitions xmlns:zzz="http://www.omg.org/spec/BPMN/20100524/MODEL"
          xmlns:zzdi="http://www.omg.org/spec/BPMN/20100524/DI"
          xmlns="http://www.omg.org/spec/DD/20100524/DC">
  <zzz:process id="P">
    <zzz:startEvent id="S"/>
  </zzz:process>
  <zzdi:BPMNDiagram id="D"><zzdi:BPMNPlane bpmnElement="P">
    <zzdi:BPMNShape bpmnElement="S"><Bounds x="0" y="0" width="36" height="36"/></zzdi:BPMNShape>
  </zzdi:BPMNPlane></zzdi:BPMNDiagram>
</zzz:definitions>
"#,
        )
        .expect("prefix-agnostic parse");
        assert!(document.find_node("S").is_some());
        let shape = &document.definitions.diagrams[0].plane.shapes[0];
        assert_eq!(shape.bounds, Bounds::new(0.0, 0.0, 36.0, 36.0));
    }

    #[test]
    fn xsi_type_overrides_task_kind() {
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                 xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <process id="P">
    <task id="T" xsi:type="serviceTask"/>
    <gateway id="G" xsi:type="bpmn2:parallelGateway" xmlns:bpmn2="http://www.omg.org/spec/BPMN/20100524/MODEL"/>
  </process>
</definitions>
"#,
        )
        .expect("parse");
        assert!(matches!(
            document.find_node("T").unwrap().kind,
            NodeKind::Task {
                task_type: TaskType::Service
            }
        ));
        assert!(matches!(
            document.find_node("G").unwrap().kind,
            NodeKind::Gateway {
                gateway_type: GatewayType::Parallel
            }
        ));
    }

    #[test]
    fn event_definitions_are_captured() {
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P">
    <startEvent id="S"><timerEventDefinition id="td"/></startEvent>
    <endEvent id="E"><terminateEventDefinition id="td2"/></endEvent>
  </process>
</definitions>
"#,
        )
        .expect("parse");
        assert!(matches!(
            document.find_node("S").unwrap().kind,
            NodeKind::StartEvent {
                definition: EventDefinition::Timer
            }
        ));
        assert!(matches!(
            document.find_node("E").unwrap().kind,
            NodeKind::EndEvent {
                definition: EventDefinition::Terminate
            }
        ));
    }

    #[test]
    fn subprocess_bodies_are_nested() {
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P">
    <subProcess id="Sub">
      <task id="Inner"/>
      <sequenceFlow id="SubFlow" sourceRef="Inner" targetRef="Inner2"/>
      <task id="Inner2"/>
    </subProcess>
  </process>
</definitions>
"#,
        )
        .expect("parse");
        assert!(document.find_node("Inner").is_some());
        assert!(document.find_sequence_flow("SubFlow").is_some());
    }

    #[test]
    fn collaboration_participants_and_message_flows() {
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <collaboration id="C">
    <participant id="PoolA" name="Worker" processRef="PA"/>
    <messageFlow id="MF1" sourceRef="PoolA" targetRef="PoolB"/>
    <participant id="PoolB" processRef="PB"/>
  </collaboration>
  <process id="PA"/>
  <process id="PB"/>
</definitions>
"#,
        )
        .expect("parse");
        let collaboration = document.collaborations().next().unwrap();
        assert_eq!(collaboration.participants.len(), 2);
        assert_eq!(
            collaboration.participants[0].process_ref.as_deref(),
            Some("PA")
        );
        assert_eq!(collaboration.message_flows.len(), 1);
    }

    #[test]
    fn malformed_and_missing_root_fail() {
        assert!(from_xml(b"not xml at all <<<").is_err());
        assert!(
            from_xml(b"<wrong xmlns=\"http://www.omg.org/spec/BPMN/20100524/MODEL\"/>").is_err()
        );
        assert!(from_xml(b"").is_err());
        assert!(from_xml(&[0xFF, 0xFE, 0x41, 0x00]).is_err());
    }

    #[test]
    fn entity_expansion_is_rejected() {
        let payload = br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <!DOCTYPE definitions [ <!ENTITY a "aaaa"> ]>
  <process id="P"><task id="T" name="&a;&a;"/></process>
</definitions>"#;
        // Either a hard error or a tolerated skip is fine; expansion is not.
        if let Ok(document) = from_xml(&payload[..])
            && let Some(task) = document.find_node("T")
        {
            assert_ne!(task.name.as_deref(), Some("aaaaaaaa"));
        }
    }

    #[test]
    fn size_limit_is_enforced() {
        let options = ParseOptions {
            max_bytes: 10,
            max_depth: 512,
        };
        let result = from_xml_with_options(MINIMAL.as_bytes(), &options);
        let error = result.expect_err("size limit");
        assert_eq!(error.diagnostic.code.as_str(), "xml.size_limit");
    }

    #[test]
    fn deeply_nested_input_is_bounded() {
        let mut payload = String::from(
            "<definitions xmlns=\"http://www.omg.org/spec/BPMN/20100524/MODEL\"><process id=\"P\"><subProcess id=\"s0\"><subProcess id=\"s1\">",
        );
        for i in 0..2000 {
            payload.push_str(&format!("<subProcess id=\"deep{i}\">"));
        }
        for _ in 0..2000 {
            payload.push_str("</subProcess>");
        }
        payload.push_str("</subProcess></subProcess></process></definitions>");
        let result = from_xml(payload.as_bytes());
        match result {
            Err(error) => assert_eq!(error.diagnostic.code.as_str(), "xml.depth_limit"),
            Ok(_) => panic!("depth limit must trigger"),
        }
    }

    fn nested_subprocesses(depth: usize) -> String {
        let mut payload = String::from(
            "<definitions xmlns=\"http://www.omg.org/spec/BPMN/20100524/MODEL\"><process id=\"P\">",
        );
        for i in 0..depth {
            payload.push_str(&format!("<subProcess id=\"d{i}\">"));
        }
        for _ in 0..depth {
            payload.push_str("</subProcess>");
        }
        payload.push_str("</process></definitions>");
        payload
    }

    #[test]
    fn depth_limit_boundary_is_inclusive() {
        // definitions + process occupy depth 1..2, so four nested
        // subprocesses sit exactly at depth 6.
        let options = |max_depth: usize| ParseOptions {
            max_bytes: 64 * 1024 * 1024,
            max_depth,
        };
        let exact = from_xml_with_options(nested_subprocesses(4).as_bytes(), &options(6));
        assert!(exact.is_ok(), "document at exactly max_depth must parse");
        let over = from_xml_with_options(nested_subprocesses(5).as_bytes(), &options(6));
        let error = over.expect_err("one level past max_depth must fail");
        assert_eq!(error.diagnostic.code.as_str(), "xml.depth_limit");
    }

    #[test]
    fn deep_subprocess_nesting_avoids_native_recursion() {
        // Regression: parsing recursed natively per `subProcess` level, so
        // deep documents overflowed the thread stack before the depth guard
        // fired (CI crashed here; local runs only passed thanks to a large
        // RUST_MIN_STACK). Nesting is heap-tracked now, so even a limit far
        // above the default parses with constant native stack use.
        const DEPTH: usize = 5_000;
        let options = ParseOptions {
            max_bytes: 64 * 1024 * 1024,
            max_depth: DEPTH + 2,
        };
        let document = from_xml_with_options(nested_subprocesses(DEPTH).as_bytes(), &options)
            .expect("deep nesting within a raised max_depth must parse");
        let Some(RootElement::Process(process)) = document.definitions.root_elements.first() else {
            panic!("expected a process root element");
        };
        let mut body: &[FlowElement] = &process.flow_elements;
        let mut levels = 0usize;
        loop {
            let FlowElement::Node(node) = body
                .first()
                .expect("subprocess body must hold the next level")
            else {
                panic!("expected a flow node at subprocess level {levels}");
            };
            let NodeKind::SubProcess { body: inner, .. } = &node.kind else {
                panic!("expected a nested subProcess at level {levels}");
            };
            levels += 1;
            let inner: &[FlowElement] = inner;
            if inner.is_empty() {
                break;
            }
            body = inner;
        }
        assert_eq!(levels, DEPTH);
    }
    #[test]
    fn unicode_and_escapes_survive() {
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="P"><task id="T" name="Kaffee &#38; Kuchen &#8212; &#x1F600;"/></process>
</definitions>
"#,
        )
        .expect("parse");
        assert_eq!(
            document.find_node("T").unwrap().name.as_deref(),
            Some("Kaffee & Kuchen — \u{1F600}")
        );
    }

    #[test]
    fn unknown_extensions_do_not_crash() {
        let document = from_xml(
            br#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                 xmlns:vendor="http://vendor.example/bpmn-ext">
  <process id="P">
    <vendor:customThing id="c" secret="42"/>
    <task id="T"><vendor:meta key="x">y</vendor:meta></task>
  </process>
</definitions>
"#,
        )
        .expect("parse");
        assert!(document.find_node("T").is_some());
        assert!(
            document
                .parse_diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::IgnoredElement)
        );
    }
}
