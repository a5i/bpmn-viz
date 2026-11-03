// Identical checks execute in Node and in a real browser against the WASM
// artifact. Pure JS: no Node APIs, so browser.html can import it too.
export function smoke({ BpmnDocument }) {
  function assert(value, message) {
    if (!value) throw new Error(message);
  }
  function throws(fn) {
    let thrown = false;
    try {
      fn();
    } catch (error) {
      assert(error instanceof Error, `Expected Error, got ${error}`);
      thrown = true;
    }
    assert(thrown, 'Expected operation to throw');
  }

  const noDi = `<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" id="defs" targetNamespace="http://example.test/wasm">
  <process id="P">
    <startEvent id="s" name="Start"/>
    <task id="t" name="Do it"/>
    <endEvent id="e" name="End"/>
    <sequenceFlow id="f1" sourceRef="s" targetRef="t"/>
    <sequenceFlow id="f2" sourceRef="t" targetRef="e"/>
  </process>
</definitions>`;

  const withDi = `<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
    xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
    xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
    xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
    id="defs2" targetNamespace="http://example.test/wasm">
  <process id="P2">
    <startEvent id="s2"/>
    <task id="t2" name="Approve"/>
    <endEvent id="e2"/>
    <sequenceFlow id="g1" sourceRef="s2" targetRef="t2"/>
    <sequenceFlow id="g2" sourceRef="t2" targetRef="e2"/>
  </process>
  <bpmndi:BPMNDiagram id="D2">
    <bpmndi:BPMNPlane bpmnElement="P2">
      <bpmndi:BPMNShape id="s2_di" bpmnElement="s2">
        <dc:Bounds x="160.0" y="100.0" width="36.0" height="36.0"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="t2_di" bpmnElement="t2">
        <dc:Bounds x="260.0" y="78.0" width="100.0" height="80.0"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="e2_di" bpmnElement="e2">
        <dc:Bounds x="420.0" y="100.0" width="36.0" height="36.0"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="g1_di" bpmnElement="g1">
        <di:waypoint x="196.0" y="118.0"/>
        <di:waypoint x="260.0" y="118.0"/>
      </bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="g2_di" bpmnElement="g2">
        <di:waypoint x="360.0" y="118.0"/>
        <di:waypoint x="420.0" y="118.0"/>
      </bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</definitions>`;

  throws(() => new BpmnDocument('not xml at all'));

  const document = new BpmnDocument(noDi);
  assert(document.diagramCount === 0, 'Model without DI must report zero diagrams');
  const inspected = document.inspect();
  assert(inspected.contractVersion === 1, 'Inspect envelope version');
  assert(inspected.processes === 1, 'One process expected');
  assert(inspected.nodesByKind.task === 1, 'One task expected');
  const validated = document.validate();
  assert(validated.errorCount === 1, 'No-DI model reports the missing-interchange error (AC-09)');
  assert(
    validated.diagnostics[0].code === 'di.missing_interchange',
    'Stable diagnostic code for missing DI',
  );
  throws(() => document.renderSvg(), 'Missing DI must throw, never fall back (AC-09)');
  const laid = document.layout();
  assert(laid.diagram.shapes.length === 3, 'Layout generates three shapes');
  assert(laid.diagram.edges.length === 2, 'Layout generates two edges');
  assert(laid.warnings.length === 0, 'Minimal layout warns nothing');
  const laidAgain = document.layout();
  assert(JSON.stringify(laid) === JSON.stringify(laidAgain), 'Layout must be deterministic');
  const laidSvg = document.renderLayoutSvg();
  assert(laidSvg.svg.startsWith('<?xml'), 'Layout render produces an SVG document');
  assert(laidSvg.svg.includes('<svg xmlns'), 'Layout render contains the svg root');
  assert(laidSvg.warnings.length === 0, 'Layout render warns nothing');

  const withDiDocument = new BpmnDocument(withDi);
  assert(withDiDocument.diagramCount === 1, 'DI model exposes one diagram');
  const rendered = withDiDocument.renderSvg();
  assert(rendered.svg.startsWith('<?xml'), 'DI render produces an SVG document');
  assert(rendered.svg.includes('<svg xmlns'), 'DI render contains the svg root');
  assert(rendered.svg === withDiDocument.renderSvg().svg, 'SVG output is deterministic');
  assert(rendered.width === 316, 'Viewport width is geometry plus default padding');
  assert(rendered.height === 100, 'Viewport height is geometry plus default padding');
  assert(rendered.warnings.length === 0, 'DI render warns nothing');
  const selected = withDiDocument.renderSvg({ diagram: 'D2' });
  assert(selected.svg === rendered.svg, 'Selection by diagram id works');
  const byIndex = withDiDocument.renderSvg({ diagramIndex: 0 });
  assert(byIndex.svg === rendered.svg, 'Selection by diagram index works');
  throws(() => withDiDocument.renderSvg({ diagram: 'missing' }), 'Unknown diagram id throws');
  // Option strictness lives in the TypeScript contract: serde-wasm-bindgen
  // cannot enforce deny_unknown_fields, so unknown keys are ignored here.
}
