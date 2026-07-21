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

  // Accept both Moqentra domain tool names and LabelU-native tool ids.
  const toolMap: Record<string, string> = {
    imageClassification: 'imageClassification',
    videoClassification: 'videoClassification',
    boundingBox: 'rectTool',
    rectTool: 'rectTool',
    polygon: 'polygonTool',
    polygonTool: 'polygonTool',
    keyPoint: 'pointTool',
    pointTool: 'pointTool',
    objectTracking: 'frameRectTool',
    frameRectTool: 'frameRectTool',
    relation: 'lineTool',
    lineTool: 'lineTool',
    cuboidTool: 'cuboidTool',
    segmentTool: 'segmentTool',
  };

  const seen = new Set<string>();
  const tools: LabelUToolConfig[] = [];
  for (const raw of ontology.tools) {
    const tool = toolMap[raw] ?? raw;
    if (!tool || seen.has(tool)) continue;
    seen.add(tool);
    tools.push({
      tool,
      config: {
        label: ontology.labels.map((l) => l.name).join(','),
        labels: ontology.labels.map((l) => ({
          name: l.name,
          color: l.color,
          value: l.id,
        })),
      },
    });
  }

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
  assetId: string;
  labelId: string;
  geometry: Record<string, number[] | undefined>;
  frame?: number;
  tool?: string;
}

export function fromLabelUAnnotations(
  labelUAnnotations: LabelUAnnotation[],
  assetId: string,
): MoqentraAnnotationPatch[] {
  return labelUAnnotations.map((a) => {
    const geometry: Record<string, number[] | undefined> = {};
    if (a.points && a.points.length > 0) {
      if (a.tool === 'rectTool' || a.type === 'rect') {
        // LabelU rects commonly use [x1,y1,x2,y2]; convert to [x,y,w,h] when possible.
        if (a.points.length >= 4) {
          const [x1, y1, x2, y2] = a.points;
          geometry.bbox = [x1, y1, Math.max(0, x2 - x1), Math.max(0, y2 - y1)];
        } else {
          geometry.points = a.points;
        }
      } else if (a.tool === 'polygonTool' || a.type === 'polygon') {
        geometry.polygon = a.points;
      } else {
        geometry.points = a.points;
      }
    }
    return {
      annotationId: a.id,
      assetId,
      labelId: a.label,
      geometry,
      frame: a.frame,
      tool: a.tool ?? a.type,
    };
  });
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
