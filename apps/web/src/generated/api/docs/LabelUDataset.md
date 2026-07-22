
# LabelUDataset


## Properties

Name | Type
------------ | -------------
`config` | [LabelUProjectConfig](LabelUProjectConfig.md)
`annotations` | { [key: string]: Array&lt;LabelUAnnotation&gt;; }

## Example

```typescript
import type { LabelUDataset } from ''

// TODO: Update the object below with actual values
const example = {
  "config": null,
  "annotations": null,
} satisfies LabelUDataset

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as LabelUDataset
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


