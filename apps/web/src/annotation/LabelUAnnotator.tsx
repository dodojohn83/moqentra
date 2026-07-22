import { Annotator, type ImageAnnotatorProps, type AnnotatorRef } from '@labelu/image-annotator-react';
import { forwardRef, useCallback, useImperativeHandle, useRef } from 'react';

export type LabelUAnnotatorRef = {
  getAnnotations: () => ReturnType<NonNullable<AnnotatorRef['getAnnotations']>>;
};

export interface LabelUAnnotatorProps extends Omit<ImageAnnotatorProps, 'samples'> {
  mediaUrl: string;
  sampleName?: string;
}

export const LabelUAnnotator = forwardRef<LabelUAnnotatorRef, LabelUAnnotatorProps>(
  ({ mediaUrl, sampleName, ...props }, ref) => {
    const annotatorRef = useRef<AnnotatorRef | null>(null);

    const handleLoad = useCallback(
      (engine: Parameters<NonNullable<ImageAnnotatorProps['onLoad']>>[0]) => {
        if (props.onLoad) {
          props.onLoad(engine);
        }
      },
      [props],
    );

    useImperativeHandle(ref, () => ({
      getAnnotations: () => annotatorRef.current?.getAnnotations(),
    }));

    const samples: ImageAnnotatorProps['samples'] = [
      {
        id: mediaUrl,
        url: mediaUrl,
        name: sampleName ?? mediaUrl,
        data: {},
      },
    ];

    return <Annotator ref={annotatorRef} {...props} samples={samples} onLoad={handleLoad} />;
  },
);

LabelUAnnotator.displayName = 'LabelUAnnotator';
