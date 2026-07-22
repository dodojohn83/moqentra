
# LabelUProjectConfig


## Properties

Name | Type
------------ | -------------
`version` | string
`mediaType` | string
`tools` | [Array&lt;LabelUToolConfig&gt;](LabelUToolConfig.md)

## Example

```typescript
import type { LabelUProjectConfig } from ''

// TODO: Update the object below with actual values
const example = {
  "version": null,
  "mediaType": null,
  "tools": null,
} satisfies LabelUProjectConfig

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as LabelUProjectConfig
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


