
# LabelUAnnotation


## Properties

Name | Type
------------ | -------------
`id` | string
`type` | string
`label` | string
`tool` | string
`frame` | number
`points` | Array&lt;number&gt;

## Example

```typescript
import type { LabelUAnnotation } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "type": null,
  "label": null,
  "tool": null,
  "frame": null,
  "points": null,
} satisfies LabelUAnnotation

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as LabelUAnnotation
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


