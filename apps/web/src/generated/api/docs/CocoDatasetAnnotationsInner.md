
# CocoDatasetAnnotationsInner


## Properties

Name | Type
------------ | -------------
`id` | number
`image_id` | number
`category_id` | number
`bbox` | Array&lt;number&gt;
`segmentation` | object
`keypoints` | object
`area` | number
`iscrowd` | number

## Example

```typescript
import type { CocoDatasetAnnotationsInner } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "image_id": null,
  "category_id": null,
  "bbox": null,
  "segmentation": null,
  "keypoints": null,
  "area": null,
  "iscrowd": null,
} satisfies CocoDatasetAnnotationsInner

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as CocoDatasetAnnotationsInner
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


