/**
 * Adapter between Moqentra's AnnotationProjectSpec/v1 and LabelU-Kit shapes.
 *
 * This module does not import LabelU packages directly; it defines the JSON
 * boundary so the front end can serialize project/ontology/task definitions to
 * the fixed LabelU format and convert LabelU annotations back to Moqentra's
 * native model before saving.
 */

export interface MoqentraOntology {
  schemaVersion: string;
  labels: Array<{
    id: string;
    name: string;
    color: string;
    parentId?: string;
    attributes?: Record<string, unknown>;
  }>;
  tools: Array<
    | 'imageClassification'
    | 'videoClassification'
    | 'rectTool'
    | 'polygonTool'
    | 'pointTool'
    | 'lineTool'
    | 'cuboidTool'
    | 'frameRectTool'
    | 'segmentTool'
  >;
}

export interface LabelUToolConfig {
  tool: string;
  config: {
    attribute?: Record<string, unknown>;
    label: string;
    color?: string;
    labels?: Array<{ name: string; color: string; value: string }>;
  };
}

export interface LabelUProjectConfig {
  version: 'v1';
  mediaType: 'image' | 'video' | 'audio';
  tools: LabelUToolConfig[];
}

export function toLabelUProjectConfig(
  taskType: string,
  ontology: MoqentraOntology,
): LabelUProjectConfig {
  const mediaType: LabelUProjectConfig['mediaType'] = taskType.startsWith('video')
    ? 'video'
    : taskType.startsWith('audio')
      ? 'audio'
      : 'image';

  const toolMap: Record<string, string> = {
    imageClassification: 'imageClassification',
    videoClassification: 'videoClassification',
    boundingBox: 'rectTool',
    polygon: 'polygonTool',
    keyPoint: 'pointTool',
    objectTracking: 'frameRectTool',
    relation: 'lineTool',
  };

  const tools: LabelUToolConfig[] = ontology.tools
    .map((tool) => toolMap[tool] || tool)
    .filter((tool) => tool !== undefined)
    .map((tool) => ({
      tool,
      config: {
        label: ontology.labels.map((l) => l.name).join(','),
        labels: ontology.labels.map((l) => ({
          name: l.name,
          color: l.color,
          value: l.id,
        })),
      },
    }));

  return { version: 'v1', mediaType, tools };
}

export interface LabelUAnnotation {
  id: string;
  type: string;
  label: string;
  points?: number[];
  tool?: string;
  frame?: number;
}

export interface MoqentraAnnotationPatch {
  annotationId: string;
  labelId: string;
  geometry: Record<string, number[] | undefined>;
  frame?: number;
}

export function fromLabelUAnnotations(
  labelUAnnotations: LabelUAnnotation[],
  assetId: string,
): MoqentraAnnotationPatch[] {
  return labelUAnnotations.map((a) => ({
    annotationId: a.id,
    labelId: a.label,
    geometry: a.points ? { points: a.points } : {},
    frame: a.frame,
  }));
}

export function maskLabelFromPayload(
  payload: Record<string, unknown>,
): Record<string, unknown> {
  const clone = { ...payload };
  delete clone.url;
  delete clone.signedUrl;
  delete clone.presignedUrl;
  delete clone.s3Key;
  delete clone.secret;
  return clone;
}
