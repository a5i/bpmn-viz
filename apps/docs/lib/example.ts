import type { RenderMode } from "@/lib/types";

export interface PlaygroundExample {
  id: string;
  title: string;
  description: string;
  challenge: string;
  /** Default render pipeline for this example. */
  mode: RenderMode;
  fileName: string;
  document: string;
  breakDocument: (text: string) => string;
}

const XML_HEADER = `<?xml version="1.0" encoding="UTF-8"?>`;

export const exampleDiDocument = `${XML_HEADER}
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"
                  xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                  xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
                  xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
                  id="Definitions_Minimal"
                  targetNamespace="http://example.test/bpmn/minimal">
  <bpmn:process id="Process_Minimal" isExecutable="false">
    <bpmn:startEvent id="StartEvent_1" name="Received">
      <bpmn:outgoing>Flow_1</bpmn:outgoing>
    </bpmn:startEvent>
    <bpmn:task id="Task_1" name="Approve request">
      <bpmn:incoming>Flow_1</bpmn:incoming>
      <bpmn:outgoing>Flow_2</bpmn:outgoing>
    </bpmn:task>
    <bpmn:endEvent id="EndEvent_1" name="Done">
      <bpmn:incoming>Flow_2</bpmn:incoming>
    </bpmn:endEvent>
    <bpmn:sequenceFlow id="Flow_1" sourceRef="StartEvent_1" targetRef="Task_1"/>
    <bpmn:sequenceFlow id="Flow_2" sourceRef="Task_1" targetRef="EndEvent_1"/>
  </bpmn:process>
  <bpmndi:BPMNDiagram id="BPMNDiagram_1">
    <bpmndi:BPMNPlane id="BPMNPlane_1" bpmnElement="Process_Minimal">
      <bpmndi:BPMNShape id="StartEvent_1_di" bpmnElement="StartEvent_1">
        <dc:Bounds x="160.0" y="100.0" width="36.0" height="36.0"/>
        <bpmndi:BPMNLabel>
          <dc:Bounds x="152.0" y="140.0" width="52.0" height="14.0"/>
        </bpmndi:BPMNLabel>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_1_di" bpmnElement="Task_1">
        <dc:Bounds x="260.0" y="78.0" width="100.0" height="80.0"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="EndEvent_1_di" bpmnElement="EndEvent_1">
        <dc:Bounds x="420.0" y="100.0" width="36.0" height="36.0"/>
        <bpmndi:BPMNLabel>
          <dc:Bounds x="424.0" y="140.0" width="28.0" height="14.0"/>
        </bpmndi:BPMNLabel>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Flow_1_di" bpmnElement="Flow_1">
        <di:waypoint x="196.0" y="118.0"/>
        <di:waypoint x="260.0" y="118.0"/>
      </bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_2_di" bpmnElement="Flow_2">
        <di:waypoint x="360.0" y="118.0"/>
        <di:waypoint x="420.0" y="118.0"/>
      </bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>
`;

// The same shapes the committed fixture pins, adapted for the layout demo:
// a model that ships no bpmndi section at all and still renders through the
// native layered engine (ADR-0006/ADR-0007).
export const exampleLayoutDocument = `${XML_HEADER}
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             id="Definitions_Order" targetNamespace="http://example.test/bpmn/order">
  <process id="Process_Order" isExecutable="false">
    <startEvent id="Start_1" name="Order placed">
      <outgoing>Flow_1</outgoing>
    </startEvent>
    <exclusiveGateway id="Gateway_1" name="In stock?">
      <incoming>Flow_1</incoming>
      <outgoing>Flow_2</outgoing>
      <outgoing>Flow_3</outgoing>
    </exclusiveGateway>
    <task id="Task_Ship" name="Ship from warehouse">
      <incoming>Flow_2</incoming>
      <outgoing>Flow_4</outgoing>
    </task>
    <task id="Task_Order" name="Order from supplier">
      <incoming>Flow_3</incoming>
      <outgoing>Flow_4</outgoing>
    </task>
    <endEvent id="End_1" name="Fulfilled">
      <incoming>Flow_4</incoming>
    </endEvent>
    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Gateway_1"/>
    <sequenceFlow id="Flow_2" sourceRef="Gateway_1" targetRef="Task_Ship"/>
    <sequenceFlow id="Flow_3" sourceRef="Gateway_1" targetRef="Task_Order"/>
    <sequenceFlow id="Flow_4" sourceRef="Task_Ship" targetRef="End_1"/>
    <sequenceFlow id="Flow_5" sourceRef="Task_Order" targetRef="End_1"/>
  </process>
</definitions>
`;

export const exampleDiagnosticsDocument = `${XML_HEADER}
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
             xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
             id="Defs_Dup" targetNamespace="http://example.test/invalid">
  <process id="Process_Dup" isExecutable="false">
    <task id="Task_Dup" name="first"/>
    <task id="Task_Dup" name="second"/>
  </process>
  <bpmndi:BPMNDiagram id="Diagram_Dup">
    <bpmndi:BPMNPlane id="Plane_Dup" bpmnElement="Process_Dup">
      <bpmndi:BPMNShape id="Task_Dup_di_1" bpmnElement="Task_Dup">
        <dc:Bounds x="10" y="10" width="100" height="80"/>
      </bpmndi:BPMNShape>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</definitions>
`;

export const exampleCollaborationDocument = `${XML_HEADER}
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
             xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
             xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
             id="Defs_Collab" targetNamespace="http://example.test/collab">
  <collaboration id="Collaboration_1">
    <participant id="Pool_Buyer" name="Buyer" processRef="Process_Buyer"/>
    <participant id="Pool_Seller" name="Seller" processRef="Process_Seller"/>
    <messageFlow id="MessageFlow_1" name="order" sourceRef="StartEvent_B" targetRef="Task_S_Receive"/>
  </collaboration>
  <process id="Process_Buyer" isExecutable="false">
    <startEvent id="StartEvent_B" name="want item">
      <outgoing>Flow_B1</outgoing>
    </startEvent>
    <endEvent id="EndEvent_B" name="ordered">
      <incoming>Flow_B1</incoming>
    </endEvent>
    <sequenceFlow id="Flow_B1" sourceRef="StartEvent_B" targetRef="EndEvent_B"/>
    <laneSet id="LaneSet_B">
      <lane id="Lane_Browsing" name="Browsing">
        <flowNodeRef>StartEvent_B</flowNodeRef>
      </lane>
      <lane id="Lane_Checkout" name="Checkout">
        <flowNodeRef>EndEvent_B</flowNodeRef>
      </lane>
    </laneSet>
  </process>
  <process id="Process_Seller" isExecutable="false">
    <receiveTask id="Task_S_Receive" name="receive order">
      <outgoing>Flow_S1</outgoing>
    </receiveTask>
    <serviceTask id="Task_S_Ship" name="ship item">
      <incoming>Flow_S1</incoming>
      <outgoing>Flow_S2</outgoing>
    </serviceTask>
    <endEvent id="EndEvent_S">
      <incoming>Flow_S2</incoming>
    </endEvent>
    <sequenceFlow id="Flow_S1" sourceRef="Task_S_Receive" targetRef="Task_S_Ship"/>
    <sequenceFlow id="Flow_S2" sourceRef="Task_S_Ship" targetRef="EndEvent_S"/>
  </process>
  <bpmndi:BPMNDiagram id="Diagram_Collab">
    <bpmndi:BPMNPlane id="Plane_Collab" bpmnElement="Collaboration_1">
      <bpmndi:BPMNShape id="Pool_Buyer_di" bpmnElement="Pool_Buyer" isHorizontal="true">
        <dc:Bounds x="160" y="60" width="520" height="160"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Lane_Browsing_di" bpmnElement="Lane_Browsing" isHorizontal="true">
        <dc:Bounds x="190" y="60" width="490" height="80"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Lane_Checkout_di" bpmnElement="Lane_Checkout" isHorizontal="true">
        <dc:Bounds x="190" y="140" width="490" height="80"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Pool_Seller_di" bpmnElement="Pool_Seller" isHorizontal="true">
        <dc:Bounds x="160" y="260" width="520" height="120"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="StartEvent_B_di" bpmnElement="StartEvent_B">
        <dc:Bounds x="240" y="82" width="36" height="36"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="EndEvent_B_di" bpmnElement="EndEvent_B">
        <dc:Bounds x="600" y="162" width="36" height="36"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Flow_B1_di" bpmnElement="Flow_B1">
        <di:waypoint x="276" y="100"/>
        <di:waypoint x="618" y="180"/>
      </bpmndi:BPMNEdge>
      <bpmndi:BPMNShape id="Task_S_Receive_di" bpmnElement="Task_S_Receive">
        <dc:Bounds x="240" y="280" width="100" height="80"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_S_Ship_di" bpmnElement="Task_S_Ship">
        <dc:Bounds x="420" y="280" width="100" height="80"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="EndEvent_S_di" bpmnElement="EndEvent_S">
        <dc:Bounds x="600" y="302" width="36" height="36"/>
      </bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Flow_S1_di" bpmnElement="Flow_S1">
        <di:waypoint x="340" y="320"/>
        <di:waypoint x="420" y="320"/>
      </bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="Flow_S2_di" bpmnElement="Flow_S2">
        <di:waypoint x="520" y="320"/>
        <di:waypoint x="600" y="320"/>
      </bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="MessageFlow_1_di" bpmnElement="MessageFlow_1">
        <di:waypoint x="258" y="118"/>
        <di:waypoint x="258" y="280"/>
      </bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</definitions>
`;

export const playgroundExamples: PlaygroundExample[] = [
  {
    id: "di",
    title: "01 · Render DI geometry",
    description: "A start → task → end process whose shapes and edge waypoints come straight from the BPMNDI section.",
    challenge: "Point Flow_1's targetRef at a missing id — the preview re-renders and the dangling reference appears under Diagnostics.",
    mode: "di",
    fileName: "approval.bpmn",
    document: exampleDiDocument,
    breakDocument: text => text.replace(`targetRef="Task_1"`, `targetRef="Task_Missing"`),
  },
  {
    id: "layout",
    title: "02 · Auto-layout a no-DI model",
    description: "This order process ships no BPMNDI section. The layered engine ranks nodes, routes flows and renders the generated geometry — validation still notes the missing DI, by design.",
    challenge: "Reorder the flows, or add a gateway branch — ranks and rows are recomputed as you type.",
    mode: "layout",
    fileName: "order.bpmn",
    document: exampleLayoutDocument,
    breakDocument: text => text.replace(`sourceRef="Gateway_1"`, `sourceRef="Gateway_Missing"`),
  },
  {
    id: "diagnostics",
    title: "03 · Diagnostics",
    description: "Two tasks share one id. Validation reports the collision with severity and source location instead of guessing.",
    challenge: "Rename the second task's id to Task_Dup_2 and watch every diagnostic clear.",
    mode: "di",
    fileName: "broken.bpmn",
    document: exampleDiagnosticsDocument,
    breakDocument: text => text.replace(`width="100" height="80"`, `width="NaN" height="80"`),
  },
  {
    id: "pools",
    title: "04 · Pools, lanes & message flows",
    description: "A collaboration with two pools, lanes and a message flow between them — rendered exactly as the DI places it.",
    challenge: "Point the message flow at Task_S_Ship and see the diagram, then check Diagnostics for what changed.",
    mode: "di",
    fileName: "collaboration.bpmn",
    document: exampleCollaborationDocument,
    breakDocument: text => text.replace(`targetRef="Task_S_Receive"/>`, `targetRef="Task_S_Ship"/>`),
  },
];

export const defaultExample = playgroundExamples[0];
